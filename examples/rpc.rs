/* Copyright (C) 2019 by Jacob Alexander
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

use capnp;
use capnp::capability::Promise;
use capnp_rpc::{pry, rpc_twoparty_capnp, twoparty, RpcSystem};
use rand::Rng;
use std::fs;
use std::io::Read;
use std::io::Write;
use std::net::ToSocketAddrs;
use std::sync::Arc;
use tokio::io::AsyncRead;
use tokio::prelude::Future;
use tokio_rustls::{rustls::ClientConfig, TlsConnector};

use hid_io_core::common_capnp::NodeType;
use hid_io_core::hidio_capnp::h_i_d_i_o_server;
use hid_io_core::protocol::hidio::*;

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

pub fn main() -> Result<(), ::capnp::Error> {
    trait Duplex: tokio::io::AsyncRead + tokio::io::AsyncWrite {};
    impl<T> Duplex for T where T: tokio::io::AsyncRead + tokio::io::AsyncWrite {}

    let mut core = ::tokio_core::reactor::Core::new()?;
    let handle = core.handle();

    let addr = LISTEN_ADDR
        .to_socket_addrs()?
        .next()
        .expect("could not parse address");
    println!("Connecting to {}", addr);

    let mut config = ClientConfig::new();
    config
        .dangerous()
        .set_certificate_verifier(Arc::new(danger::NoCertificateVerification {}));
    let config = TlsConnector::from(Arc::new(config));

    let domain = webpki::DNSNameRef::try_from_ascii_str("localhost").unwrap();

    let socket = ::tokio_core::net::TcpStream::connect(&addr, &handle);
    let tls_handshake = socket.and_then(|socket| {
        socket.set_nodelay(true).unwrap();
        config.connect(domain, socket)
    });

    let stream = core.run(tls_handshake).unwrap();
    let (reader, writer) = stream.split();

    let network = Box::new(twoparty::VatNetwork::new(
        reader,
        writer,
        rpc_twoparty_capnp::Side::Client,
        Default::default(),
    ));
    let mut rpc_system = RpcSystem::new(network, None);
    let hidio_server: h_i_d_i_o_server::Client =
        rpc_system.bootstrap(rpc_twoparty_capnp::Side::Server);

    let _rpc_disconnector = rpc_system.get_disconnector();
    handle.spawn(rpc_system.map_err(|e| println!("{}", e)));

    // Display server version information
    let request = hidio_server.version_request();
    core.run(request.send().promise.and_then(|response| {
        let value = response.get().unwrap().get_version().unwrap();
        println!("Version:    {}", value.get_version().unwrap());
        println!("Buildtime:  {}", value.get_buildtime().unwrap());
        println!("Serverarch: {}", value.get_serverarch().unwrap());
        println!("Compiler:   {}", value.get_compilerversion().unwrap());
        Promise::ok(())
    }))?;

    // Lookup key location
    let auth_key_file = {
        let request = hidio_server.key_request();
        core.run(request.send().promise.and_then(|response| {
            let value = response.get().unwrap().get_key().unwrap();
            Promise::ok(value.get_auth_key_path().unwrap().to_string())
        }))?
    };
    println!("Key Path:   {}", auth_key_file);

    // Lookup key
    let auth_key = fs::read_to_string(auth_key_file)?;
    println!("Key:        {}", auth_key);

    // Lookup uid
    let uid = {
        let request = hidio_server.id_request();
        core.run(request.send().promise.and_then(|response| {
            let value = response.get().unwrap().get_id();
            Promise::ok(value)
        }))?
    };
    println!("Id:         {}", uid);

    // Make authentication request
    let hidio = {
        let mut request = hidio_server.auth_request();
        let mut info = request.get().get_info()?;
        let mut rng = rand::thread_rng();
        info.set_type(NodeType::HidioApi);
        info.set_name("RPC Test");
        info.set_serial(&format!("{:x}", rng.gen::<u64>()));
        info.set_id(uid);
        request.get().set_key(&auth_key);
        request.send().pipeline.get_port()
    };

    let nodes_resp = {
        let request = hidio.nodes_request();
        core.run(request.send().promise)?
    };
    let nodes = nodes_resp.get()?.get_nodes()?;

    let args: Vec<_> = std::env::args().collect();
    let nid = match args.get(1) {
        Some(n) => n.parse().unwrap(),
        None => {
            let keyboards: Vec<_> = nodes
                .iter()
                .filter(|n| n.get_type().unwrap() == NodeType::UsbKeyboard)
                .collect();
            if keyboards.len() == 1 {
                let n = keyboards[0];
                println!("Registering to {}", format_node(n));
                n.get_id()
            } else {
                println!();
                for n in nodes {
                    println!(" * {} - {}", n.get_id(), format_node(n));
                }

                print!("Please choose a device: ");
                std::io::stdout().flush()?;

                let mut n = String::new();
                std::io::stdin().read_line(&mut n)?;
                n.trim().parse().unwrap()
            }
        }
    };

    // TODO: Select from command line arg
    let device = nodes.iter().find(|n| n.get_id() == nid);
    if device.is_none() {
        eprintln!("Could not find node: {}", nid);
        std::process::exit(1);
    }
    let device = device.unwrap();

    let register_resp = {
        let node = device.get_node()?;
        let request = node.register_request();
        core.run(request.send().promise)?
    };
    let ok = register_resp.get()?.get_ok();
    if !ok {
        println!("Could not register to node");
        std::process::exit(1);
    }

    println!("READY");
    let (vt_tx, vt_rx) = std::sync::mpsc::channel::<u8>();
    std::thread::spawn(move || loop {
        for byte in std::io::stdin().lock().bytes() {
            if let Ok(b) = byte {
                vt_tx.send(b).unwrap();
            } else {
                println!("Lost stdin");
                std::process::exit(2);
            }
        }
    });

    loop {
        let mut vt_buf = vec![];
        loop {
            match vt_rx.try_recv() {
                Ok(c) => {
                    vt_buf.push(c);
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    break;
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    println!("Lost socket");
                    ::std::process::exit(1);
                }
            }
        }

        if !vt_buf.is_empty() {
            use hid_io_core::common_capnp::destination::commands::Which::*;
            if let Ok(commands) = device.get_commands().which() {
                match commands {
                    UsbKeyboard(node) => {
                        let node = node?;
                        let _command_resp = {
                            let mut request = node.cli_command_request();
                            request.get().set_foobar(&String::from_utf8(vt_buf)?);
                            core.run(request.send().promise)?
                        };
                    }
                    HostMacro(_node) => {}
                    HidioPacket(_node) => {}
                }
            }
        }

        use hid_io_core::hidio_capnp::h_i_d_i_o::signal::type_::{
            HidioPacket, HostMacro, UsbKeyboard,
        };
        use hid_io_core::hidiowatcher_capnp::h_i_d_i_o_watcher::signal::{
            DevicePacket, HostPacket,
        };
        use hid_io_core::usbkeyboard_capnp::u_s_b_keyboard::signal::{KeyEvent, ScanCodeEvent};

        let mut req = hidio.signal_request();
        req.get().set_time(27); // TODO: Timing
        let result = core.run(req.send().promise.and_then(|response| {
            let signals = pry!(pry!(response.get()).get_signal());
            for signal in signals.iter() {
                let p = pry!(signal.get_type().which());
                match p {
                    UsbKeyboard(p) => {
                        let p = pry!(pry!(p).which());
                        match p {
                            KeyEvent(p) => {
                                let p = pry!(p);
                                let _e = p.get_event();
                                let id = p.get_id();
                                println!("Key event: {}", id);
                            }
                            ScanCodeEvent(_p) => {}
                        }
                    }
                    HostMacro(_p) => {}
                    HidioPacket(p) => {
                        let p = pry!(pry!(p).which());
                        match p {
                            HostPacket(_p) => {}
                            DevicePacket(p) => {
                                let p = pry!(p);
                                let data = pry!(p.get_data()).iter().collect::<Vec<u8>>();
                                let id: HIDIOCommandID =
                                    unsafe { std::mem::transmute(p.get_id() as u16) };
                                match id {
                                    HIDIOCommandID::Terminal => {
                                        pry!(std::io::stdout().write_all(&data));
                                        pry!(std::io::stdout().flush());
                                    }
                                    HIDIOCommandID::HostMacro => {}
                                    // ...
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }
            Promise::ok(())
        }));
        if let Err(e) = result {
            match e.kind {
                capnp::ErrorKind::Disconnected => {
                    // TODO: Reconnect
                    std::process::exit(3);
                }
                capnp::ErrorKind::Overloaded => {}
                _ => {
                    eprintln!("Error: {}", e.description);
                }
            }
        }
    }
    /*
    TODO This is how to cleanly disconnect
    core.run(rpc_disconnector)?;
    Ok(())
    */
}
