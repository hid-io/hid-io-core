/* Copyright (C) 2020 by Jacob Alexander
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

use capnp;
use capnp::capability::Promise;
use capnp_rpc::{pry, rpc_twoparty_capnp, twoparty, RpcSystem};
use clap::{App, Arg, SubCommand};
use futures::{AsyncReadExt, FutureExt};
use hid_io_core::built_info;
use hid_io_core::common_capnp::NodeType;
use hid_io_core::hidio_capnp;
use hid_io_core::hidio_capnp::hid_io_server;
use hid_io_core::keyboard_capnp;
use hid_io_core::logging::setup_logging_lite;
use log::*;
use rand::Rng;
use std::fs;
use std::io::Write;
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
                _ => {}
            }
        }

        Promise::ok(())
    }
}

#[tokio::main]
pub async fn main() -> Result<(), ::capnp::Error> {
    setup_logging_lite().ok();
    tokio::task::LocalSet::new().run_until(try_main()).await
}

async fn try_main() -> Result<(), ::capnp::Error> {
    let version_info = format!(
        "{}{} - {}",
        built_info::PKG_VERSION,
        built_info::GIT_VERSION.map_or_else(|| "".to_owned(), |v| format!(" (git {})", v)),
        built_info::PROFILE,
    );
    let after_info = format!(
        "{} ({}) -> {} ({})",
        built_info::RUSTC_VERSION,
        built_info::HOST,
        built_info::TARGET,
        built_info::BUILT_TIME_UTC,
    );

    // Parse arguments
    let matches = App::new("hid-io-core tool")
        .version(version_info.as_str())
        .author(built_info::PKG_AUTHORS)
        .about(format!("\n{}", built_info::PKG_DESCRIPTION).as_str())
        .after_help(after_info.as_str())
        .arg(
            Arg::with_name("serial")
                .short("s")
                .long("serial")
                .value_name("SERIAL")
                .help("Serial number of device (may include spaces, remember to quote).")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("list")
                .short("l")
                .long("list")
                .help("Lists currently connected hid-io enabled devices."),
        )
        .subcommand(SubCommand::with_name("flash").about("Attempt to enable flash mode on device"))
        .subcommand(SubCommand::with_name("sleep").about("Attempt to enable sleep mode on device"))
        .get_matches();

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
            info.set_name("Device Tool");
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

        // List device nodes
        if matches.is_present("list") {
            let devices: Vec<_> = nodes
                .iter()
                .filter(|n| {
                    n.get_type().unwrap() == NodeType::UsbKeyboard
                        || n.get_type().unwrap() == NodeType::BleKeyboard
                })
                .collect();
            println!(" * <uid> - <NodeType>: [<VID>:<PID>-<Usage Page>:<Usage>] [<Vendor>] <Name> (<Serial>)");
            for n in devices {
                println!(" * {} - {}", n.get_id(), format_node(n));
            }
            return Ok(());
        }

        // Serial is used to specify the device (if necessary)
        let mut serial = "".to_string();

        let nid = match matches.value_of("serial") {
            Some(n) => {
                serial = n.to_string();

                let serial_matched: Vec<_> = nodes
                    .iter()
                    .filter(|n| n.get_serial().unwrap() == serial)
                    .collect();

                if serial_matched.len() == 1 {
                    let n = serial_matched[0];
                    println!("Registering to {}", format_node(n));
                    n.get_id()
                } else {
                    eprintln!("Could not find: {}", serial);
                    std::process::exit(1);
                }
            }
            None => {
                let id;

                let serial_matched: Vec<_> = nodes
                    .iter()
                    .filter(|n| n.get_serial().unwrap() == serial)
                    .collect();
                // First attempt to match serial number
                if serial != "" && serial_matched.len() == 1 {
                    let n = serial_matched[0];
                    println!("Registering to {}", format_node(n));
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
                    if serial == "" && keyboards.len() == 1 {
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
        //serial = format!("{}", device.get_serial().unwrap());

        match matches.subcommand() {
            ("flash", Some(_)) => {
                // Flash mode command
                if let Ok(nodetype) = device.get_node().which() {
                    match nodetype {
                        hid_io_core::common_capnp::destination::node::Which::Keyboard(node) => {
                            let node = node?;

                            let flash_mode_resp = {
                                // Cast/transform keyboard node to a hidio node
                                let request = hidio_capnp::node::Client {
                                    client: node.client,
                                }
                                .flash_mode_request();
                                match request.send().promise.await {
                                    Ok(response) => response,
                                    Err(e) => {
                                        eprintln!("Flash Mode request failed: {}", e);
                                        ::std::process::exit(1);
                                    }
                                }
                            };
                            // TODO Fully implement flash mode sequence
                            if flash_mode_resp
                                .get()
                                .unwrap()
                                .get_status()
                                .unwrap()
                                .has_success()
                            {
                                println!("Flash mode set");
                            }
                            // TODO Implement errors
                        }
                        _ => {}
                    }
                }
            }
            ("sleep", Some(_)) => {
                // Sleep mode command
                if let Ok(nodetype) = device.get_node().which() {
                    match nodetype {
                        hid_io_core::common_capnp::destination::node::Which::Keyboard(node) => {
                            let node = node?;

                            let sleep_mode_resp = {
                                // Cast/transform keyboard node to a hidio node
                                let request = hidio_capnp::node::Client {
                                    client: node.client,
                                }
                                .sleep_mode_request();
                                match request.send().promise.await {
                                    Ok(response) => response,
                                    Err(e) => {
                                        eprintln!("Sleep Mode request failed: {}", e);
                                        ::std::process::exit(1);
                                    }
                                }
                            };
                            // TODO Fully implement flash mode sequence
                            if sleep_mode_resp
                                .get()
                                .unwrap()
                                .get_status()
                                .unwrap()
                                .has_success()
                            {
                                println!("Sleep mode set");
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {
                warn!("No command specified");
            }
        }

        return Ok(());
    }
    /*
    _rpc_disconnector.await?;
    Ok(())
    */
}
