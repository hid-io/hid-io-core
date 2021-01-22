#![cfg(feature = "api")]
/* Copyright (C) 2019-2020 by Jacob Alexander
 * Copyright (C) 2019 by Rowan Decker
 *
 * This file is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This file is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this file.  If not, see <http://www.gnu.org/licenses/>.
 */

extern crate tokio;

use capnp::capability::Promise;
use capnp_rpc::{rpc_twoparty_capnp, twoparty, RpcSystem};
use futures::{AsyncReadExt, FutureExt};
use hid_io_core::common_capnp::NodeType;
use hid_io_core::hidio_capnp::hid_io;
use hid_io_core::hidio_capnp::hid_io_server;
use hid_io_protocol::HidIoCommandID;
use rand::Rng;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::fs;
use std::net::ToSocketAddrs;
use std::sync::Arc;
use tokio_rustls::{rustls::ClientConfig, TlsConnector};

const LISTEN_ADDR: &str = "localhost:7185";

mod danger {
    pub struct NoCertificateVerification {}

    impl rustls::ServerCertVerifier for NoCertificateVerification {
        fn verify_server_cert(
            &self,
            _roots: &rustls::RootCertStore,
            _certs: &[rustls::Certificate],
            _hostname: webpki::DNSNameRef<'_>,
            _ocsp: &[u8],
        ) -> Result<rustls::ServerCertVerified, rustls::TLSError> {
            Ok(rustls::ServerCertVerified::assertion())
        }
    }
}

fn format_node(node: hid_io_core::common_capnp::destination::Reader<'_>) -> String {
    format!(
        "{}: {} ({})",
        node.get_type().unwrap(),
        node.get_name().unwrap_or(""),
        node.get_serial().unwrap_or(""),
    )
}

struct Node {
    type_: NodeType,
    _name: String,
    _serial: String,
}

struct NodesSubscriberImpl {
    nodes_lookup: HashMap<u64, Node>,
    start_time: std::time::Instant,
}

impl NodesSubscriberImpl {
    fn new() -> NodesSubscriberImpl {
        let nodes_lookup: HashMap<u64, Node> = HashMap::new();
        let start_time = std::time::Instant::now();

        NodesSubscriberImpl {
            nodes_lookup,
            start_time,
        }
    }

    fn format_packet(&mut self, packet: hid_io::packet::Reader<'_>) -> String {
        let mut datastr = "".to_string();
        for b in packet.get_data().unwrap().iter() {
            datastr += &format!("{:02x}", b);
        }
        let datalen = packet.get_data().unwrap().len();
        let src = packet.get_src();
        let src_node_type = if src == 0 {
            "All".to_string()
        } else {
            if let Some(n) = self.nodes_lookup.get(&src) {
                format!("{:?}", n.type_)
            } else {
                format!("{:?}", NodeType::Unknown)
            }
        };

        let dst = packet.get_dst();
        let dst_node_type = if dst == 0 {
            "All".to_string()
        } else {
            if let Some(n) = self.nodes_lookup.get(&dst) {
                format!("{:?}", n.type_)
            } else {
                format!("{:?}", NodeType::Unknown)
            }
        };

        // TODO (HaaTa): decode packets to show fields
        if datalen == 0 {
            format!(
                "{} - {:?}: {}:{}->{}:{} ({:?}:{}) Len:{}",
                self.start_time.elapsed().as_millis(),
                packet.get_type().unwrap(),
                src,
                src_node_type,
                dst,
                dst_node_type,
                HidIoCommandID::try_from(packet.get_id()).unwrap_or(HidIoCommandID::Unused),
                packet.get_id(),
                datalen,
            )
        } else {
            format!(
                "{} - {:?}: {}:{}->{}:{} ({:?}:{}) Len:{}\n\t{}",
                self.start_time.elapsed().as_millis(),
                packet.get_type().unwrap(),
                src,
                src_node_type,
                dst,
                dst_node_type,
                HidIoCommandID::try_from(packet.get_id()).unwrap_or(HidIoCommandID::Unused),
                packet.get_id(),
                datalen,
                datastr,
            )
        }
    }
}

impl hid_io::nodes_subscriber::Server for NodesSubscriberImpl {
    fn nodes_update(
        &mut self,
        params: hid_io::nodes_subscriber::NodesUpdateParams,
        _results: hid_io::nodes_subscriber::NodesUpdateResults,
    ) -> Promise<(), capnp::Error> {
        // Re-create nodes_lookup on each update
        self.nodes_lookup = HashMap::new();

        println!("nodes_update: ");
        for n in capnp_rpc::pry!(capnp_rpc::pry!(params.get()).get_nodes()) {
            println!("{} - {}", n.get_id(), format_node(n));
            self.nodes_lookup.insert(
                n.get_id(),
                Node {
                    type_: n.get_type().unwrap(),
                    _name: n.get_name().unwrap_or("").to_string(),
                    _serial: n.get_serial().unwrap_or("").to_string(),
                },
            );
        }
        Promise::ok(())
    }

    fn hidio_watcher(
        &mut self,
        params: hid_io::nodes_subscriber::HidioWatcherParams,
        _results: hid_io::nodes_subscriber::HidioWatcherResults,
    ) -> Promise<(), capnp::Error> {
        println!(
            "{}",
            self.format_packet(capnp_rpc::pry!(capnp_rpc::pry!(params.get()).get_packet()))
        );
        Promise::ok(())
    }
}

#[tokio::main]
pub async fn main() -> Result<(), ::capnp::Error> {
    tokio::task::LocalSet::new().run_until(try_main()).await
}

async fn try_main() -> Result<(), ::capnp::Error> {
    let addr = LISTEN_ADDR
        .to_socket_addrs()?
        .next()
        .expect("could not parse address");
    println!("Connecting to {}", addr);

    let mut config = ClientConfig::new();
    config
        .dangerous()
        .set_certificate_verifier(Arc::new(danger::NoCertificateVerification {}));
    let connector = TlsConnector::from(Arc::new(config));

    let domain = webpki::DNSNameRef::try_from_ascii_str("localhost").unwrap();

    loop {
        let stream = match tokio::net::TcpStream::connect(&addr).await {
            Ok(stream) => stream,
            Err(e) => {
                println!("Failed to connect ({}): {}", addr, e);
                tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
                continue;
            }
        };
        stream.set_nodelay(true)?;
        let stream = connector.connect(domain, stream).await?;

        let (reader, writer) =
            tokio_util::compat::Tokio02AsyncReadCompatExt::compat(stream).split();

        let network = Box::new(twoparty::VatNetwork::new(
            reader,
            writer,
            rpc_twoparty_capnp::Side::Client,
            Default::default(),
        ));
        let mut rpc_system = RpcSystem::new(network, None);
        let hidio_server: hid_io_server::Client =
            rpc_system.bootstrap(rpc_twoparty_capnp::Side::Server);

        let _rpc_disconnector = rpc_system.get_disconnector();
        tokio::task::spawn_local(Box::pin(rpc_system.map(|_| ())));

        // Display server version information
        let request = hidio_server.version_request();
        let response = request.send().promise.await?;
        let value = response.get().unwrap().get_version().unwrap();
        println!("Version:    {}", value.get_version().unwrap());
        println!("Buildtime:  {}", value.get_buildtime().unwrap());
        println!("Serverarch: {}", value.get_serverarch().unwrap());
        println!("Compiler:   {}", value.get_compilerversion().unwrap());

        // Lookup key location
        let auth_key_file = {
            let request = hidio_server.key_request();
            let response = request.send().promise.await?;
            let value = response.get().unwrap().get_key().unwrap();
            value.get_auth_key_path().unwrap().to_string()
        };
        println!("Key Path:   {}", auth_key_file);

        // Lookup key
        let auth_key = fs::read_to_string(auth_key_file)?;
        println!("Key:        {}", auth_key);

        // Lookup uid
        let uid = {
            let request = hidio_server.id_request();
            let response = request.send().promise.await?;
            let value = response.get().unwrap().get_id();
            value
        };
        println!("Id:         {}", uid);

        // Make authentication request
        let hidio = {
            let mut request = hidio_server.auth_request();
            let mut info = request.get().get_info()?;
            let mut rng = rand::thread_rng();
            info.set_type(NodeType::HidioApi);
            info.set_name("watchnodes");
            info.set_serial(&format!(
                "{:x} - pid:{}",
                rng.gen::<u64>(),
                std::process::id()
            ));
            info.set_id(uid);
            request.get().set_key(&auth_key);
            request.send().pipeline.get_port()
        };

        // Subscribe to nodeswatcher
        let nodes_subscription = capnp_rpc::new_client(NodesSubscriberImpl::new());
        let mut request = hidio.subscribe_nodes_request();
        request.get().set_subscriber(nodes_subscription);
        let _callback = request.send().promise.await;

        loop {
            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;

            // Check if the server is still alive
            let request = hidio_server.alive_request();
            if let Err(e) = request.send().promise.await {
                println!("Dead: {}", e);
                // Break the subscription loop and attempt to reconnect
                break;
            }
        }
    }
    /*
    rpc_disconnector.await?;
    Ok(())
    */
}
