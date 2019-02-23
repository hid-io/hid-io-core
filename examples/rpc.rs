#![feature(await_macro, async_await, futures_api)]

#[macro_use]
extern crate tokio;

use capnp;

use capnp::capability::Promise;
use capnp_rpc::{pry, rpc_twoparty_capnp, twoparty, RpcSystem};
use tokio::io::AsyncRead;
use tokio::prelude::Future;

use hid_io::api::{load_certs, load_private_key};
use hid_io::common_capnp::NodeType;
use hid_io::hidio_capnp::h_i_d_i_o_server;

use std::fs;
use std::io::BufReader;
use std::net::ToSocketAddrs;
use std::sync::Arc;
use tokio::prelude::future::{ok, join_all};
use tokio_rustls::{rustls::ClientConfig, TlsConnector};

const USE_SSL: bool = false;

pub fn main() {
    try_main().unwrap();
}

fn try_main() -> Result<(), ::capnp::Error> {
    let host = "localhost";
    let addr = format!("{}:7185", host)
        .to_socket_addrs()
        .unwrap()
        .next()
        .expect("could not parse address");
    println!("Connecting to {}", addr);

    let ssl_config = if USE_SSL {
        let mut pem = BufReader::new(fs::File::open("test-ca/rsa/ca.cert").unwrap());
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

    let mut runtime = ::tokio::runtime::current_thread::Runtime::new().unwrap();
    let stream = runtime.block_on(socket).unwrap(); // TODO: Connection refused message
    let (reader, writer) = stream.split();

    let network = Box::new(twoparty::VatNetwork::new(
        reader,
        writer,
        rpc_twoparty_capnp::Side::Client,
        Default::default(),
    ));

    //tokio::run_async(async move
    //{
    let mut rpc_system = RpcSystem::new(network, None);
    let hidio_server: h_i_d_i_o_server::Client =
        rpc_system.bootstrap(rpc_twoparty_capnp::Side::Server);
    runtime.spawn(rpc_system.map_err(|_e| ()));

        use rand::Rng;
        let hidio = {
            let mut request = hidio_server.basic_request();
            let mut info = request.get().get_info().unwrap();
            let mut rng = rand::thread_rng();
            info.set_type(NodeType::HidioScript);
            info.set_name("RPC Test");
            info.set_serial(&format!("{:x}", rng.gen::<u64>()));
            request.send().pipeline.get_port()
        };

        let nodes_resp = {
                let request = hidio.nodes_request();
                runtime.block_on(request.send().promise).unwrap()
        };
        let nodes = nodes_resp.get().unwrap().get_nodes().unwrap();

            for n in nodes {
                /*let is_registered = n.get_node().unwrap().is_registered_request().send();
                //let registered = runtime.block_on(is_registered.promise).unwrap().get().unwrap().get_ok();
                is_registered.promise.and_then(|resp| {
                    let ok = resp.get().unwrap().get_ok();
                    println!("OK: {}", ok);
                    Promise::ok(())
                });

                let suffix = if is_registered {
                    "[REGISTERED]"
                } else {
                    ""
                };*/
                let suffix = "";
                println!(
                    "Node {} - {}: {} ({}) {}",
                    n.get_id(),
                    n.get_type().unwrap(),
                    n.get_name().unwrap_or(""),
                    n.get_serial().unwrap_or(""),
                    suffix,
                );
            }

        // TODO: Select from command line arg
        let device = nodes.get(0);
        
        let register_resp = {
                let node = device.get_node().unwrap();
                let request = node.register_request();
                runtime.block_on(request.send().promise).unwrap()
        };
        let ok = register_resp.get().unwrap().get_ok();
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
                        },
                        Err(std::sync::mpsc::TryRecvError::Empty) => {
                            break;
                        }
                        Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                                println!("Lost socket");
                                ::std::process::exit(1);
                        },
                }
            }

            if !vt_buf.is_empty() {
                // FIXME
                //let foo = device.which();
                //println!("foo {}", foo);
                //let request = device.cli_command_request();
                // device.exec_call(HIDIOCommandID::Terminal, vt_buf);
                
                use hid_io::common_capnp::destination::commands::Which::*;
                let commands = device.get_commands().which().unwrap();
                match commands {
                    UsbKeyboard(node) => {
                        let node = node.unwrap();
                        let command_resp = {
                            let mut request = node.cli_command_request();
                            request.get().set_foobar(&String::from_utf8(vt_buf).unwrap());
                            runtime.block_on(request.send().promise).unwrap()
                        };
                    }
                    _ => {}
                }
            }

        use hid_io::hidio_capnp::h_i_d_i_o::signal::type_::{UsbKeyboard, HostMacro, HidioPacket};
        use hid_io::usbkeyboard_capnp::u_s_b_keyboard::signal::{KeyEvent, ScanCodeEvent};
        use hid_io::hidiowatcher_capnp::h_i_d_i_o_watcher::signal::{HostPacket, DevicePacket};


            let mut req = hidio.signal_request();
            req.get().set_time(27);
            runtime.block_on(req.send().promise.and_then(|response| {
                let signals = pry!(pry!(response.get()).get_signal());
                for signal in signals.iter() {
                let p = pry!(signal.get_type().which());
                match p {
                    UsbKeyboard(p) => {
                        let p = pry!(pry!(p).which());
                        match p {
                            KeyEvent(p) => {
                                let p = pry!(p);
                                let e = p.get_event();
                                let id = p.get_id();
                            },
                            ScanCodeEvent(p) => {
                            }
                        }
                    },
                    HostMacro(p) => {

                    },
                    HidioPacket(p) => {
                        let p = pry!(pry!(p).which());
                        match p {
                            HostPacket(p) => {
                                //println!("HOST {}", pry!(p));
                            },
                            DevicePacket(p) => {
                                let p = pry!(p);
                                let data = p.get_data().unwrap().iter().collect::<Vec<u8>>();
                                //println!("DEVICE {}", p);
                                match p.get_id() {
                                    0x22 => {
                                        use std::io::Write;
                                        //mailbox.send_ack(device, received.id, vec![]);
                                        std::io::stdout().write_all(&data).unwrap();
                                        std::io::stdout().flush().unwrap();
                                    },
                                    _ => {}
                                }
                            }
                        }
                    },
                    _ => {}
                }
                }
                Promise::ok(())
            }));
        }

        /*loop {
            use std::io::{self, BufRead, Write};
            print!("> ");
            io::stdout().flush().unwrap();

            // Block for user input
            let mut line = String::new();
            io::stdin().lock().read_line(&mut line).unwrap();
            let args: Vec<&str> = line.trim().split(" ").collect();

            match args[0] {
                "ls" => {
                    let req = hidio.nodes_request();
                    runtime.block_on(req.send().promise.and_then(|response| {
                        let nodes = pry!(pry!(response.get()).get_nodes());
                        for n in nodes.iter() {
                            println!("AREISNARS");
                            let is_registered = n.get_node().unwrap().is_registered_request().send();
                            //let registered = runtime.block_on(is_registered.promise).unwrap().get().unwrap().get_ok();
                            is_registered.promise.and_then(|resp| {
                                let ok = resp.get().unwrap().get_ok();
                                println!("OK: {}", ok);
                                Promise::ok(())
                            });

                            /*let suffix = if is_registered {
                                "[REGISTERED]"
                            } else {
                                ""
                            };*/
                            let suffix = "";
                            println!(
                                "Node {} - {}: {} ({}) {}",
                                n.get_id(),
                                n.get_type().unwrap(),
                                n.get_name().unwrap_or(""),
                                n.get_serial().unwrap_or(""),
                                suffix,
                            );
                        }
                        Promise::ok(())
                    }));
                },
                "r" => {
                    let id: u32 = args[1].parse().unwrap();
                    //tokio::run_async(async move {
                        let req = hidio.nodes_request();
                        //req.send().promise.and_then(|response| {
                        //let response = await!(req.send().promise).unwrap();
                        /*let nodes = response.get().unwrap().get_nodes().unwrap();
                        let n = nodes.iter().find(|n| n.get_id() == id);
                        let n = n.unwrap();*/
                        //if let Some(n) = n {
                            /*Promise::ok(await!(
                            n.get_node().unwrap().register_request().send().promise.and_then(|_| {
                                println!("Registered");
                                Promise::ok(())
                            })).unwrap());*/
                        /*} else {
                            println!("Unknown node");
                            Promise::ok(())
                        }*/
                        //Promise::ok(())
                        //})
                    //});
                },
                "" => {},
                _ => println!("Unknown Command '{}'", args[0]),

            }

            //runtime.run();
        }*/
    //});

    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }

    Ok(())
}
