/* Copyright (C) 2017 by Jacob Alexander
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

// ----- Crates -----

// ----- Modules -----
mod auth;

pub use crate::common_capnp::*;
pub use crate::devicefunction_capnp::*;
pub use crate::hidio_capnp::*;
pub use crate::hidiowatcher_capnp::*;
pub use crate::hostmacro_capnp::*;
pub use crate::usbkeyboard_capnp::*;

use capnp::capability::Promise;
use capnp::Error;
use capnp_rpc::{pry, rpc_twoparty_capnp, twoparty, RpcSystem};
use tokio::io::AsyncRead;
use tokio::prelude::{Future, Stream};
use tokio::runtime::current_thread;

use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::io::BufReader;
use std::rc::Rc;
use std::sync::Arc;
use tokio::prelude::future::{lazy, ok};
use tokio_rustls::{
    rustls::{AllowAnyAuthenticatedClient, RootCertStore, ServerConfig},
    TlsAcceptor,
};

const USE_SSL: bool = false;

// ----- Functions -----

impl std::fmt::Display for NodeType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            NodeType::HidioDaemon => write!(f, "HidioDaemon"),
            NodeType::HidioScript => write!(f, "HidioScript"),
            NodeType::UsbKeyboard => write!(f, "UsbKeyboard"),
        }
    }
}

enum AuthLevel {
    Basic,
    Secure,
    Debug,
}

struct Endpoint {
    type_: NodeType,
    name: String,
    serial: String,
    id: u64,
}

struct HIDIOMaster {
    nodes: Vec<Endpoint>,
    connections: HashMap<u64, Vec<u64>>,
    last_uid: u64,
}

impl HIDIOMaster {
    fn new() -> HIDIOMaster {
        HIDIOMaster {
            nodes: Vec::new(),
            connections: HashMap::new(),
            last_uid: 0,
        }
    }
}

use crate::hidio_capnp::h_i_d_i_o_server::*;

struct HIDIOServerImpl {
    master: Rc<RefCell<HIDIOMaster>>,
    uid: u64,
}

impl HIDIOServerImpl {
    fn new(master: Rc<RefCell<HIDIOMaster>>, uid: u64) -> HIDIOServerImpl {
        HIDIOServerImpl { master, uid }
    }

    fn create_connection(&mut self, mut node: Endpoint, auth: AuthLevel) -> h_i_d_i_o::Client {
        {
            let mut m = self.master.borrow_mut();
            let id = m.last_uid + 1;
            m.last_uid = id;
            node.id = id;
            m.connections.get_mut(&self.uid).unwrap().push(node.id);
            m.nodes.push(node);
        }

        h_i_d_i_o::ToClient::new(HIDIOImpl::new(Rc::clone(&self.master), auth))
            .into_client::<::capnp_rpc::Server>()
    }
}

impl h_i_d_i_o_server::Server for HIDIOServerImpl {
    fn basic(&mut self, params: BasicParams, mut results: BasicResults) -> Promise<(), Error> {
        let info = pry!(pry!(params.get()).get_info());
        let node = Endpoint {
            type_: info.get_type().unwrap(),
            name: info.get_name().unwrap().to_string(),
            serial: info.get_serial().unwrap().to_string(),
            id: 0,
        };
        results
            .get()
            .set_port(self.create_connection(node, AuthLevel::Secure));
        Promise::ok(())
    }

    fn auth(&mut self, params: AuthParams, mut results: AuthResults) -> Promise<(), Error> {
        use crate::api::auth::UAC;

        // TODO: Auth implementation selection
        let authenticator = auth::DummyAuth {};

        if authenticator.auth() {
            let info = pry!(pry!(params.get()).get_info());
            let node = Endpoint {
                type_: info.get_type().unwrap(),
                name: info.get_name().unwrap().to_string(),
                serial: info.get_serial().unwrap().to_string(),
                id: 0,
            };
            results
                .get()
                .set_port(self.create_connection(node, AuthLevel::Secure));
            Promise::ok(())
        } else {
            Promise::err(Error {
                kind: capnp::ErrorKind::Failed,
                description: "Authentication denied".to_string(),
            })
        }
    }
}

use crate::hidio_capnp::h_i_d_i_o::*;

struct HIDIOImpl {
    master: Rc<RefCell<HIDIOMaster>>,
    auth: AuthLevel,
}

impl HIDIOImpl {
    fn new(master: Rc<RefCell<HIDIOMaster>>, auth: AuthLevel) -> HIDIOImpl {
        HIDIOImpl { master, auth }
    }

    fn init_signal(mut signal: h_i_d_i_o::signal::Builder<'_>) {
        signal.set_time(15);

        {
            let mut source = signal.reborrow().init_source();
            source.set_type(NodeType::UsbKeyboard);
            source.set_name("Test usbkeyboard signal source");
            source.set_serial("SERIAL NUMBER!");
            source.set_id(1234567);
        }

        {
            let typ = signal.init_type();
            let kbd = typ.init_usb_keyboard();
            let mut event = kbd.init_key_event();
            event.set_event(KeyEventState::Press);
            event.set_id(32);
        }
    }
}

impl h_i_d_i_o::Server for HIDIOImpl {
    fn signal(&mut self, _params: SignalParams, mut results: SignalResults) -> Promise<(), Error> {
        results.get().set_time(10);

        let signal = results.get().init_signal(1).get(0);
        HIDIOImpl::init_signal(signal);

        Promise::ok(())
    }

    fn nodes(&mut self, _params: NodesParams, mut results: NodesResults) -> Promise<(), Error> {
        let nodes = &self.master.borrow().nodes;
        let mut result = results.get().init_nodes(nodes.len() as u32);
        for (i, n) in nodes.iter().enumerate() {
            let mut node = result.reborrow().get(i as u32);
            node.set_type(n.type_);
            node.set_name(&n.name);
            node.set_serial(&n.serial);
            node.set_id(n.id);
            node.set_node(
                h_i_d_i_o_node::ToClient::new(HIDIONodeImpl::HostIO(false))
                    .into_client::<::capnp_rpc::Server>(),
            );
        }
        Promise::ok(())
    }
}

enum HIDIONodeImpl {
    USBKeyboard(bool),
    HostIO(bool),
}

use crate::common_capnp::h_i_d_i_o_node::*;
impl h_i_d_i_o_node::Server for HIDIONodeImpl {
    fn register(
        &mut self,
        _params: RegisterParams,
        mut results: RegisterResults,
    ) -> Promise<(), Error> {
        let ok: bool = match self {
            HIDIONodeImpl::USBKeyboard(connected) | HIDIONodeImpl::HostIO(connected) => {
                *connected = true;
                true
            }
        };

        results.get().set_ok(ok);
        Promise::ok(())
    }

    fn is_registered(
        &mut self,
        _params: IsRegisteredParams,
        mut results: IsRegisteredResults,
    ) -> Promise<(), Error> {
        let connected: bool = match self {
            HIDIONodeImpl::USBKeyboard(c) | HIDIONodeImpl::HostIO(c) => *c,
        };
        results.get().set_ok(connected);
        Promise::ok(())
    }
}

pub fn load_certs(filename: &str) -> Vec<rustls::Certificate> {
    let certfile = fs::File::open(filename).expect("cannot open certificate file");
    let mut reader = BufReader::new(certfile);
    rustls::internal::pemfile::certs(&mut reader).unwrap()
}

pub fn load_private_key(filename: &str) -> rustls::PrivateKey {
    let rsa_keys = {
        let keyfile = fs::File::open(filename).expect("cannot open private key file");
        let mut reader = BufReader::new(keyfile);
        rustls::internal::pemfile::rsa_private_keys(&mut reader)
            .expect("file contains invalid rsa private key")
    };

    let pkcs8_keys = {
        let keyfile = fs::File::open(filename).expect("cannot open private key file");
        let mut reader = BufReader::new(keyfile);
        rustls::internal::pemfile::pkcs8_private_keys(&mut reader)
            .expect("file contains invalid pkcs8 private key (encrypted keys not supported)")
    };

    if !pkcs8_keys.is_empty() {
        pkcs8_keys[0].clone()
    } else {
        assert!(!rsa_keys.is_empty());
        rsa_keys[0].clone()
    }
}

/// Cap'n'Proto API Initialization
/// Sets up a localhost socket to deal with localhost-only API usages
/// Requires TLS TODO
/// Some API usages may require external authentication to validate trustworthiness
pub fn initialize() {
    info!("Initializing api...");

    use std::net::ToSocketAddrs;
    let addr = "localhost:7185"
        .to_socket_addrs()
        .unwrap()
        .next()
        .expect("could not parse address");
    let socket = ::tokio::net::TcpListener::bind(&addr).expect("Failed to open socket");
    println!("API: Listening on {}", addr);

    let ssl_config = if USE_SSL {
        let mut client_auth_roots = RootCertStore::empty();
        let roots = load_certs("./test-ca/rsa/end.fullchain");
        for root in &roots {
            client_auth_roots.add(&root).unwrap();
        }
        let client_auth = AllowAnyAuthenticatedClient::new(client_auth_roots);

        let mut config = ServerConfig::new(client_auth);
        config
            .set_single_cert(roots, load_private_key("./test-ca/rsa/end.key"))
            .unwrap();
        let config = TlsAcceptor::from(Arc::new(config));
        Some(config)
    } else {
        None
    };

    trait Duplex: tokio::io::AsyncRead + tokio::io::AsyncWrite {};
    impl<T> Duplex for T where T: tokio::io::AsyncRead + tokio::io::AsyncWrite {}

    let connections = socket.incoming().map(|socket| {
        socket.set_nodelay(true).unwrap();
        let c: Box<Future<Item = Box<_>, Error = std::io::Error>> =
            if let Some(config) = &ssl_config {
                Box::new(config.accept(socket).and_then(|a| {
                    let accept: Box<Duplex> = Box::new(a);
                    ok(accept)
                }))
            } else {
                let accept: Box<Duplex> = Box::new(socket);
                Box::new(ok(accept))
            };

        c
    });

    // TODO: This will be generated from connected devices
    let mut nodes = vec![Endpoint {
        type_: NodeType::UsbKeyboard,
        name: "Test Keyboard".to_string(),
        serial: "1467".to_string(),
        id: 78500,
    }];

    let mut m = HIDIOMaster::new();
    m.nodes.append(&mut nodes);

    let master = Rc::new(RefCell::new(m));

    let done = connections.for_each(|connect_promise| {
        connect_promise.and_then(|socket| {
            let (reader, writer) = socket.split();

            // Client connected, create node
            let network = twoparty::VatNetwork::new(
                reader,
                writer,
                rpc_twoparty_capnp::Side::Server,
                Default::default(),
            );
            current_thread::spawn(lazy(|| {
                let rc = Rc::clone(&master);
                let uid = {
                    let mut m = rc.borrow_mut();
                    m.last_uid += 1;
                    let uid = m.last_uid;
                    m.connections.insert(uid, vec![]);
                    uid
                };

                let hidio_server =
                    h_i_d_i_o_server::ToClient::new(HIDIOServerImpl::new(Rc::clone(&rc), uid))
                        .into_client::<::capnp_rpc::Server>();
                let rpc_system = RpcSystem::new(Box::new(network), Some(hidio_server.client));

                rpc_system
                    .map_err(|e| println!("error: {:?}", e))
                    .and_then(move |_| {
                        println!("DONE: {}", uid);

                        // Client disconnected, delete node
                        let connected_nodes = rc.borrow().connections.get(&uid).unwrap().clone();
                        rc.borrow_mut()
                            .nodes
                            .retain(|x| !connected_nodes.contains(&x.id));
                        Ok(())
                    })
            }));
            Ok(())
        })
    });

    current_thread::block_on_all(done).unwrap();
}
