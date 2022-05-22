#![cfg(feature = "api")]
/* Copyright (C) 2019-2022 by Jacob Alexander
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

use capnp_rpc::{rpc_twoparty_capnp, twoparty, RpcSystem};
use futures::{AsyncReadExt, FutureExt};
use hid_io_core::common_capnp::NodeType;
use hid_io_core::hidio_capnp::hid_io_server;
use hid_io_core::logging::setup_logging_lite;
use rand::Rng;
use std::fs;
use std::net::ToSocketAddrs;
use std::sync::Arc;
use tokio_rustls::{rustls::ClientConfig, TlsConnector};

const LISTEN_ADDR: &str = "localhost:7185";

mod danger {
    use std::time::SystemTime;
    use tokio_rustls::rustls::{Certificate, ServerName};

    pub struct NoCertificateVerification {}

    impl rustls::client::ServerCertVerifier for NoCertificateVerification {
        fn verify_server_cert(
            &self,
            _end_entity: &Certificate,
            _intermediates: &[Certificate],
            _server_name: &ServerName,
            _scts: &mut dyn Iterator<Item = &[u8]>,
            _ocsp_response: &[u8],
            _now: SystemTime,
        ) -> Result<rustls::client::ServerCertVerified, rustls::Error> {
            Ok(rustls::client::ServerCertVerified::assertion())
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

#[tokio::main]
pub async fn main() -> Result<(), ::capnp::Error> {
    setup_logging_lite().ok();
    tokio::task::LocalSet::new().run_until(try_main()).await
}

async fn try_main() -> Result<(), ::capnp::Error> {
    let addr = LISTEN_ADDR
        .to_socket_addrs()?
        .next()
        .expect("could not parse address");
    println!("Connecting to {}", addr);

    let config = ClientConfig::builder()
        .with_safe_defaults()
        .with_custom_certificate_verifier(Arc::new(danger::NoCertificateVerification {}))
        .with_no_client_auth();
    let connector = TlsConnector::from(Arc::new(config));

    let domain = rustls::ServerName::try_from("localhost").unwrap();

    let stream = tokio::net::TcpStream::connect(&addr).await?;
    stream.set_nodelay(true)?;
    let stream = connector.connect(domain, stream).await?;

    let (reader, writer) = tokio_util::compat::TokioAsyncReadCompatExt::compat(stream).split();

    let network = Box::new(twoparty::VatNetwork::new(
        reader,
        writer,
        rpc_twoparty_capnp::Side::Client,
        Default::default(),
    ));
    let mut rpc_system = RpcSystem::new(network, None);
    let hidio_server: hid_io_server::Client =
        rpc_system.bootstrap(rpc_twoparty_capnp::Side::Server);

    let rpc_disconnector = rpc_system.get_disconnector();
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
        info.set_name("lsnodes");
        info.set_serial(&format!(
            "{:x} - pid:{}",
            rng.gen::<u64>(),
            std::process::id()
        ));
        info.set_id(uid);
        request.get().set_key(&auth_key);
        request.send().pipeline.get_port()
    };

    let nodes_resp = {
        let request = hidio.nodes_request();
        request.send().promise.await?
    };
    let nodes = nodes_resp.get()?.get_nodes()?;

    println!();
    for n in nodes {
        println!(" * {} - {}", n.get_id(), format_node(n));
    }

    rpc_disconnector.await?;
    Ok(())
}
