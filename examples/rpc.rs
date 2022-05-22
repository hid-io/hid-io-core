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

use capnp::capability::Promise;
use capnp_rpc::{pry, rpc_twoparty_capnp, twoparty, RpcSystem};
use futures::{AsyncReadExt, FutureExt};
use hid_io_core::common_capnp::NodeType;
use hid_io_core::hidio_capnp;
use hid_io_core::hidio_capnp::hid_io_server;
use hid_io_core::keyboard_capnp;
use rand::Rng;
use std::fs;
use std::io::Read;
use std::io::Write;
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

struct KeyboardSubscriberImpl;

impl keyboard_capnp::keyboard::subscriber::Server for KeyboardSubscriberImpl {
    fn update(
        &mut self,
        params: keyboard_capnp::keyboard::subscriber::UpdateParams,
        _results: keyboard_capnp::keyboard::subscriber::UpdateResults,
    ) -> Promise<(), ::capnp::Error> {
        let signal = pry!(pry!(params.get()).get_signal());

        // Only read cli messages
        if let Ok(signaltype) = signal.get_data().which() {
            match signaltype {
                hid_io_core::keyboard_capnp::keyboard::signal::data::Which::Cli(cli) => {
                    let cli = cli.unwrap();
                    print!("{}", cli.get_output().unwrap());
                    std::io::stdout().flush().unwrap();
                }
                hid_io_core::keyboard_capnp::keyboard::signal::data::Which::Manufacturing(res) => {
                    let res = res.unwrap();
                    println!("{}:{} => ", res.get_cmd(), res.get_arg());
                    match res.get_cmd() {
                        3 => match res.get_arg() {
                            2 => {
                                let split = res.get_data().unwrap().len() / 2 / 6;
                                let mut tmp = vec![];
                                let mut pos = 0;
                                for byte in res.get_data().unwrap() {
                                    tmp.push(byte);
                                    if tmp.len() == 2 {
                                        print!("{:>4} ", u16::from_le_bytes([tmp[0], tmp[1]]));
                                        tmp.clear();
                                        pos += 1;
                                        if pos % split == 0 {
                                            println!();
                                        }
                                    }
                                }
                                println!();
                            }
                            _ => {
                                for byte in res.get_data().unwrap() {
                                    print!("{} ", byte);
                                }
                                println!();
                            }
                        },
                        _ => {
                            for byte in res.get_data().unwrap() {
                                print!("{} ", byte);
                            }
                            println!();
                        }
                    }
                    std::io::stdout().flush().unwrap();
                }
                _ => {}
            }
        }

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

    let config = ClientConfig::builder()
        .with_safe_defaults()
        .with_custom_certificate_verifier(Arc::new(danger::NoCertificateVerification {}))
        .with_no_client_auth();
    let connector = TlsConnector::from(Arc::new(config));

    let domain = rustls::ServerName::try_from("localhost").unwrap();

    // Serial is used for automatic reconnection if hid-io goes away and comes back
    let mut serial = "".to_string();

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
        let stream = connector.connect(domain.clone(), stream).await?;

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
            info.set_name("RPC Test");
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
            request.send().promise.await.unwrap()
        };
        let nodes = nodes_resp.get()?.get_nodes()?;

        let args: Vec<_> = std::env::args().collect();
        let nid = match args.get(1) {
            Some(n) => n.parse().unwrap(),
            None => {
                let id;

                let serial_matched: Vec<_> = nodes
                    .iter()
                    .filter(|n| n.get_serial().unwrap() == serial)
                    .collect();
                // First attempt to match serial number
                if !serial.is_empty() && serial_matched.len() == 1 {
                    let n = serial_matched[0];
                    println!("Re-registering to {}", format_node(n));
                    id = n.get_id();
                } else {
                    let keyboards: Vec<_> = nodes
                        .iter()
                        .filter(|n| {
                            n.get_type().unwrap() == NodeType::UsbKeyboard
                                || n.get_type().unwrap() == NodeType::BleKeyboard
                        })
                        .collect();

                    // Next, if serial number is unset and there is only one keyboard, automatically attach
                    if serial.is_empty() && keyboards.len() == 1 {
                        let n = keyboards[0];
                        println!("Registering to {}", format_node(n));
                        id = n.get_id();
                    // Otherwise display a list of keyboard nodes
                    } else {
                        println!();
                        for n in keyboards {
                            println!(" * {} - {}", n.get_id(), format_node(n));
                        }

                        print!("Please choose a device: ");
                        std::io::stdout().flush()?;

                        let mut n = String::new();
                        std::io::stdin().read_line(&mut n)?;
                        id = n.trim().parse().unwrap();
                    }
                }
                id
            }
        };

        let device = nodes.iter().find(|n| n.get_id() == nid);
        if device.is_none() {
            eprintln!("Could not find node: {}", nid);
            std::process::exit(1);
        }
        let device = device.unwrap();
        serial = device.get_serial().unwrap().to_string();

        // Build subscription callback
        let subscription = capnp_rpc::new_client(KeyboardSubscriberImpl);

        // Subscribe to cli messages
        let subscribe_req = {
            let node = match device.get_node().which().unwrap() {
                hid_io_core::common_capnp::destination::node::Which::Keyboard(n) => n.unwrap(),
                hid_io_core::common_capnp::destination::node::Which::Daemon(_) => {
                    std::process::exit(1);
                }
            };
            let mut request = node.subscribe_request();
            let mut params = request.get();
            params.set_subscriber(subscription);

            // Build list of options
            let mut options = params.init_options(1);
            let mut cli_option = options.reborrow().get(0);
            cli_option.set_type(keyboard_capnp::keyboard::SubscriptionOptionType::CliOutput);
            request
        };
        let _callback = subscribe_req.send().promise.await.unwrap();

        println!("READY");
        let (vt_tx, mut vt_rx) = tokio::sync::mpsc::channel::<u8>(100);
        std::thread::spawn(move || loop {
            for byte in std::io::stdin().lock().bytes() {
                if let Ok(b) = byte {
                    if let Err(e) = vt_tx.blocking_send(b) {
                        println!("Restarting stdin loop: {}", e);
                        return;
                    }
                } else {
                    println!("Lost stdin");
                    std::process::exit(2);
                }
            }
        });

        loop {
            let mut vt_buf = vec![];
            // Await the first byte
            match vt_rx.recv().await {
                Some(c) => {
                    vt_buf.push(c);
                }
                None => {
                    println!("Lost socket");
                    ::std::process::exit(1);
                }
            }
            // Loop over the rest of the buffer
            loop {
                match vt_rx.try_recv() {
                    Ok(c) => {
                        vt_buf.push(c);
                    }
                    Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {
                        // Done, can begin sending cli message to device
                        break;
                    }
                    Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                        println!("Lost socket (buffer)");
                        ::std::process::exit(1);
                    }
                }
            }

            if let Ok(nodetype) = device.get_node().which() {
                match nodetype {
                    hid_io_core::common_capnp::destination::node::Which::Keyboard(node) => {
                        let node = node?;
                        let _command_resp = {
                            // Cast/transform keyboard node to a hidio node
                            let mut request = hidio_capnp::node::Client {
                                client: node.client,
                            }
                            .cli_command_request();
                            request.get().set_command(&String::from_utf8(vt_buf)?);
                            match request.send().promise.await {
                                Ok(response) => response,
                                Err(e) => {
                                    println!("Dead: {}", e);
                                    break;
                                }
                            }
                        };
                    }
                    hid_io_core::common_capnp::destination::node::Which::Daemon(_node) => {}
                }
            }
        }
    }
    /*
    _rpc_disconnector.await?;
    Ok(())
    */
}
