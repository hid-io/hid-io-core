#![feature(await_macro, async_await, futures_api)]

extern crate tokio;

use capnp;

use capnp::capability::Promise;
use capnp_rpc::{pry, rpc_twoparty_capnp, twoparty, RpcSystem};
use tokio::io::AsyncRead;
use tokio::prelude::Future;

use hid_io::api::{load_certs, load_private_key};
use hid_io::common_capnp::NodeType;
use hid_io::hidio_capnp::h_i_d_i_o_server;
use hid_io::protocol::hidio::*;

use std::fs;
use std::io::BufReader;
use std::net::ToSocketAddrs;
use std::sync::Arc;
use tokio::prelude::future::ok;
use tokio_rustls::{rustls::ClientConfig, TlsConnector};

const USE_SSL: bool = false;

pub fn main() -> Result<(), ::capnp::Error> {
    let host = "localhost";
    let addr = format!("{}:7185", host)
        .to_socket_addrs()?
        .next()
        .expect("could not parse address");
    println!("Connecting to {}", addr);

    let ssl_config = if USE_SSL {
        let mut pem = BufReader::new(fs::File::open("test-ca/rsa/ca.cert")?);
        let mut config = ClientConfig::new();
        config.root_store.add_pem_file(&mut pem).unwrap();
        config.set_single_client_cert(
            load_certs("test-ca/rsa/client.cert"),
            load_private_key("test-ca/rsa/client.key"),
        );
        let config = TlsConnector::from(Arc::new(config));
        Some(config)
    } else {
        None
    };

    trait Duplex: tokio::io::AsyncRead + tokio::io::AsyncWrite {};
    impl<T> Duplex for T where T: tokio::io::AsyncRead + tokio::io::AsyncWrite {}

    let socket = ::tokio::net::TcpStream::connect(&addr).and_then(|socket| {
        socket.set_nodelay(true).unwrap();
        let c: Box<Future<Item = Box<_>, Error = std::io::Error>> =
            if let Some(config) = &ssl_config {
                let domain = webpki::DNSNameRef::try_from_ascii_str(host).unwrap();
                Box::new(config.connect(domain, socket).and_then(|a| {
                    let accept: Box<Duplex> = Box::new(a);
                    ok(accept)
                }))
            } else {
                let accept: Box<Duplex> = Box::new(socket);
                Box::new(ok(accept))
            };

        c
    });

    let mut runtime = ::tokio::runtime::current_thread::Runtime::new()?;
    let stream = runtime.block_on(socket)?; // TODO: Connection refused message
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
    runtime.spawn(rpc_system.map_err(|_e| ()));

    use rand::Rng;
    let hidio = {
        let mut request = hidio_server.auth_request();
        let mut info = request.get().get_info()?;
        let mut rng = rand::thread_rng();
        info.set_type(NodeType::HidioScript);
        info.set_name("RPC Test");
        info.set_serial(&format!("{:x}", rng.gen::<u64>()));
        request.send().pipeline.get_port()
    };

    let nodes_resp = {
        let request = hidio.nodes_request();
        runtime.block_on(request.send().promise)?
    };
    let nodes = nodes_resp.get()?.get_nodes()?;

    let args: Vec<_> = std::env::args().collect();
    let nid = match args.get(1) {
        Some(n) => n.parse().unwrap(),
        None => {
            println!("");
            for n in nodes {
                let suffix = "";
                println!(
                    " * {} - {}: {} ({}) {}",
                    n.get_id(),
                    n.get_type().unwrap(),
                    n.get_name().unwrap_or(""),
                    n.get_serial().unwrap_or(""),
                    suffix,
                );
            }

            use std::io::Write;
            print!("Please choose a device: ");
            std::io::stdout().flush()?;

            let mut n = String::new();
            std::io::stdin().read_line(&mut n)?;
            n.trim().parse().unwrap()
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
        runtime.block_on(request.send().promise)?
    };
    let ok = register_resp.get()?.get_ok();
    if !ok {
        println!("Could not register to node");
        std::process::exit(1);
    }

    println!("READY");
    let (vt_tx, vt_rx) = std::sync::mpsc::channel::<u8>();
    std::thread::spawn(move || {
        use std::io::Read;
        loop {
            for byte in std::io::stdin().lock().bytes() {
                if let Ok(b) = byte {
                    vt_tx.send(b).unwrap();
                } else {
                    println!("Lost stdin");
                    std::process::exit(2);
                }
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
            use hid_io::common_capnp::destination::commands::Which::*;
            if let Ok(commands) = device.get_commands().which() {
                match commands {
                    UsbKeyboard(node) => {
                        let node = node?;
                        let _command_resp = {
                            let mut request = node.cli_command_request();
                            request.get().set_foobar(&String::from_utf8(vt_buf)?);
                            runtime.block_on(request.send().promise)?
                        };
                    }
                    _ => {}
                }
            }
        }

        use hid_io::hidio_capnp::h_i_d_i_o::signal::type_::{HidioPacket, HostMacro, UsbKeyboard};
        use hid_io::hidiowatcher_capnp::h_i_d_i_o_watcher::signal::{DevicePacket, HostPacket};
        use hid_io::usbkeyboard_capnp::u_s_b_keyboard::signal::{KeyEvent, ScanCodeEvent};

        let mut req = hidio.signal_request();
        req.get().set_time(27); // TODO: Timing
        let result = runtime.block_on(req.send().promise.and_then(|response| {
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
                                        use std::io::Write;
                                        pry!(std::io::stdout().write_all(&data));
                                        pry!(std::io::stdout().flush());
                                    }
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
}
