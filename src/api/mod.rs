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

use crate::device::{HIDIOMailbox, HIDIOMessage};
use crate::protocol::hidio::HIDIOCommandID;
use crate::RUNNING;
use capnp::capability::Promise;
use capnp::Error;
use capnp_rpc::{pry, rpc_twoparty_capnp, twoparty, RpcSystem};
use std::sync::atomic::Ordering;
use tokio::io::AsyncRead;
use tokio::prelude::{Future, Stream};
use tokio::runtime::current_thread;

use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::io::BufReader;
use std::rc::Rc;
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex, RwLock};
use tokio::prelude::future::{lazy, ok};
use tokio_rustls::{
    rustls::{AllowAnyAuthenticatedClient, RootCertStore, ServerConfig},
    TlsAcceptor,
};

// TODO: Use config file or cli args
const LISTEN_ADDR: &str = "localhost:7185";
const USE_SSL: bool = false;

#[cfg(debug_assertions)]
const AUTH_LEVEL: AuthLevel = AuthLevel::Debug;

#[cfg(not(debug_assertions))]
const AUTH_LEVEL: AuthLevel = AuthLevel::Secure;

use lazy_static::lazy_static;
lazy_static! {
    pub static ref WRITERS_RC: Arc<Mutex<Vec<std::sync::mpsc::Sender<HIDIOMessage>>>> =
        Arc::new(Mutex::new(vec![]));
    pub static ref READERS_RC: Arc<Mutex<Vec<std::sync::mpsc::Receiver<HIDIOMessage>>>> =
        Arc::new(Mutex::new(vec![]));
}

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
impl std::fmt::Debug for NodeType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

#[derive(Clone, Copy)]
pub enum AuthLevel {
    Basic,
    Secure,
    Debug,
}

#[derive(Debug, Clone)]
pub struct Endpoint {
    pub type_: NodeType,
    pub name: String,
    pub serial: String,
    pub id: u64,
}

struct HIDIOMaster {
    nodes: Vec<Endpoint>,
    devices: Arc<RwLock<Vec<Endpoint>>>,
    connections: HashMap<u64, Vec<u64>>,
    last_uid: u64,
}

impl HIDIOMaster {
    fn new(devices: Arc<RwLock<Vec<Endpoint>>>) -> HIDIOMaster {
        HIDIOMaster {
            nodes: Vec::new(),
            devices,
            connections: HashMap::new(),
            last_uid: 0,
        }
    }
}

use crate::hidio_capnp::h_i_d_i_o_server::*;

struct HIDIOServerImpl {
    master: Rc<RefCell<HIDIOMaster>>,
    uid: u64,
    incoming: Rc<HIDIOMailbox>,
}

impl HIDIOServerImpl {
    fn new(master: Rc<RefCell<HIDIOMaster>>, uid: u64, incoming: HIDIOMailbox) -> HIDIOServerImpl {
        let incoming = Rc::new(incoming);
        HIDIOServerImpl {
            master,
            uid,
            incoming,
        }
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

        println!("Connection authed!");
        h_i_d_i_o::ToClient::new(HIDIOImpl::new(
            Rc::clone(&self.master),
            Rc::clone(&self.incoming),
            auth,
        ))
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
        info!("New capnp node: {:?}", node);
        results
            .get()
            .set_port(self.create_connection(node, AuthLevel::Basic));
        Promise::ok(())
    }

    fn auth(&mut self, params: AuthParams, mut results: AuthResults) -> Promise<(), Error> {
        use crate::api::auth::UAC;

        // TODO: Auth implementation selection
        // TODO: Request desired level from node
        let authenticator = auth::DummyAuth {};
        let auth = AUTH_LEVEL;

        let info = pry!(pry!(params.get()).get_info());
        let node = Endpoint {
            type_: info.get_type().unwrap(),
            name: info.get_name().unwrap().to_string(),
            serial: info.get_serial().unwrap().to_string(),
            id: 0,
        };
        info!("New capnp node: {:?}", node);

        if authenticator.auth(&node, auth) {
            results.get().set_port(self.create_connection(node, auth));
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
    registered: Rc<RefCell<HashMap<u64, bool>>>,
    incoming: Rc<HIDIOMailbox>,
}

impl HIDIOImpl {
    fn new(
        master: Rc<RefCell<HIDIOMaster>>,
        incoming: Rc<HIDIOMailbox>,
        auth: AuthLevel,
    ) -> HIDIOImpl {
        HIDIOImpl {
            master,
            auth,
            registered: Rc::new(RefCell::new(HashMap::new())),
            incoming,
        }
    }

    fn init_signal(&self, mut signal: h_i_d_i_o::signal::Builder<'_>, message: HIDIOMessage) {
        signal.set_time(15);

        {
            let master = self.master.borrow();
            let devices = &master.devices.read().unwrap();
            let device = devices
                .iter()
                .find(|d| d.id.to_string() == message.device)
                .unwrap();

            let mut source = signal.reborrow().init_source();
            source.set_type(device.type_);
            source.set_name(&device.name);
            source.set_serial(&device.serial);
            source.set_id(device.id);
        }

        {
            let typ = signal.init_type();

            // TODO: Multiple packet types
            let p = typ.init_hidio_packet();
            let mut p = p.init_device_packet();
            p.set_id(message.message.id as u16);
            let mut d = p.init_data(message.message.data.len() as u32);
            for i in 0..message.message.data.len() {
                d.set(i as u32, message.message.data[i]);
            }
        }
    }
}

impl h_i_d_i_o::Server for HIDIOImpl {
    fn signal(&mut self, _params: SignalParams, mut results: SignalResults) -> Promise<(), Error> {
        let incoming = &self.incoming;
        match &self.auth {
            AuthLevel::Debug => {
                if let Some(message) = incoming.recv_psuedoblocking() {
                    results.get().set_time(10);
                    let signal = results.get().init_signal(1).get(0);
                    self.init_signal(signal, message);
                    Promise::ok(())
                } else {
                    Promise::err(capnp::Error {
                        kind: capnp::ErrorKind::Overloaded,
                        description: "No data".to_string(),
                    })
                }
            }
            _ => Promise::err(capnp::Error {
                kind: capnp::ErrorKind::Failed,
                description: "Insufficient authorization level".to_string(),
            }),
        }
    }

    fn nodes(&mut self, _params: NodesParams, mut results: NodesResults) -> Promise<(), Error> {
        let master = self.master.borrow();
        let nodes = &master.nodes;
        let devices = &master.devices.read().unwrap();
        let mut result = results
            .get()
            .init_nodes((nodes.len() + devices.len()) as u32);
        for (i, n) in nodes.iter().chain(devices.iter()).enumerate() {
            let mut node = result.reborrow().get(i as u32);
            node.set_type(n.type_);
            node.set_name(&n.name);
            node.set_serial(&n.serial);
            node.set_id(n.id);
            node.set_node(
                h_i_d_i_o_node::ToClient::new(HIDIONodeImpl::new(
                    Rc::clone(&self.registered),
                    n.id,
                ))
                .into_client::<::capnp_rpc::Server>(),
            );
            let mut commands = node.reborrow().init_commands();
            commands.set_usb_keyboard(
                u_s_b_keyboard::commands::ToClient::new(HIDIOKeyboardNodeImpl::new(
                    n.id,
                    self.auth,
                    Rc::clone(&self.incoming),
                ))
                .into_client::<::capnp_rpc::Server>(),
            );
        }
        Promise::ok(())
    }
}

struct HIDIOKeyboardNodeImpl {
    id: u64,
    auth: AuthLevel,
    incoming: Rc<HIDIOMailbox>,
}
impl HIDIOKeyboardNodeImpl {
    fn new(id: u64, auth: AuthLevel, incoming: Rc<HIDIOMailbox>) -> HIDIOKeyboardNodeImpl {
        HIDIOKeyboardNodeImpl { id, auth, incoming }
    }
}
use u_s_b_keyboard::commands::*;
impl u_s_b_keyboard::commands::Server for HIDIOKeyboardNodeImpl {
    fn cli_command(
        &mut self,
        params: CliCommandParams,
        _results: CliCommandResults,
    ) -> Promise<(), Error> {
        match self.auth {
            AuthLevel::Secure | AuthLevel::Debug => {
                let params = params.get().unwrap();
                let cmd = params.get_foobar().unwrap();
                self.incoming.send_command(
                    self.id.to_string(),
                    HIDIOCommandID::Terminal,
                    cmd.as_bytes().to_vec(),
                );
                Promise::ok(())
            }
            _ => Promise::err(capnp::Error {
                kind: capnp::ErrorKind::Failed,
                description: "Insufficient authorization level".to_string(),
            }),
        }
    }
}

struct HIDIONodeImpl {
    registered_nodes: Rc<RefCell<HashMap<u64, bool>>>,
    id: u64,
}

impl HIDIONodeImpl {
    fn new(registered_nodes: Rc<RefCell<HashMap<u64, bool>>>, id: u64) -> HIDIONodeImpl {
        HIDIONodeImpl {
            registered_nodes,
            id,
        }
    }
}

use crate::common_capnp::h_i_d_i_o_node::*;
impl h_i_d_i_o_node::Server for HIDIONodeImpl {
    fn register(
        &mut self,
        _params: RegisterParams,
        mut results: RegisterResults,
    ) -> Promise<(), Error> {
        info!("Registering node {}", self.id);
        self.registered_nodes
            .borrow_mut()
            .entry(self.id)
            .and_modify(|e| *e = true)
            .or_insert(true);
        results.get().set_ok(true);
        Promise::ok(())
    }

    fn is_registered(
        &mut self,
        _params: IsRegisteredParams,
        mut results: IsRegisteredResults,
    ) -> Promise<(), Error> {
        let nodes = self.registered_nodes.borrow();
        let registered = nodes.get(&self.id).unwrap_or(&false);
        results.get().set_ok(*registered);
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
pub fn initialize(mailbox: HIDIOMailbox) {
    info!("Initializing api...");

    use std::net::ToSocketAddrs;
    let addr = LISTEN_ADDR
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

    let connections = socket.incoming().map(move |socket| {
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

    let nodes = mailbox.nodes.clone();
    let m = HIDIOMaster::new(nodes.clone());

    let master = Rc::new(RefCell::new(m));

    use stream_cancel::{StreamExt, Tripwire};

    let mut rt = tokio::runtime::current_thread::Runtime::new().unwrap();
    let (trigger, tripwire) = Tripwire::new();
    let done = connections
        .take_until(tripwire)
        .for_each(move |connect_promise| {
            info!("New connection");
            let rc = Rc::clone(&master);

            let (hidapi_writer, hidapi_reader) = channel::<HIDIOMessage>();
            // TODO: poll reader too?
            let (sink, mailbox) = HIDIOMailbox::from_sender(hidapi_writer.clone(), nodes.clone());
            {
                let mut writers = WRITERS_RC.lock().unwrap();
                (*writers).push(sink);
                let mut readers = READERS_RC.lock().unwrap();
                (*readers).push(hidapi_reader);
            }

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
                    let uid = {
                        let mut m = rc.borrow_mut();
                        m.last_uid += 1;
                        let uid = m.last_uid;
                        m.connections.insert(uid, vec![]);
                        uid
                    };

                    let hidio_server = h_i_d_i_o_server::ToClient::new(HIDIOServerImpl::new(
                        Rc::clone(&rc),
                        uid,
                        mailbox,
                    ))
                    .into_client::<::capnp_rpc::Server>();
                    let rpc_system = RpcSystem::new(Box::new(network), Some(hidio_server.client));

                    rpc_system
                        .map_err(|e| eprintln!("error: {:?}", e))
                        .and_then(move |_| {
                            info!("connection closed: {}", uid);

                            // Client disconnected, delete node
                            let connected_nodes = &rc.borrow().connections[&uid].clone();
                            rc.borrow_mut()
                                .nodes
                                .retain(|x| !connected_nodes.contains(&x.id));
                            Ok(())
                        })
                }));
                Ok(())
            })
        })
        .map_err(|_| ());

    std::thread::spawn(move || {
        loop {
            if !RUNNING.load(Ordering::SeqCst) {
                break;
            }
            let message = mailbox.recv_psuedoblocking();
            if let Some(message) = message {
                warn!(" <<< YOU GOT MAIL >>> {:?}", message);
                let writers = WRITERS_RC.lock().unwrap();
                for writer in (*writers).iter() {
                    writer.send(message.clone()).unwrap();
                }
            }

            let readers = READERS_RC.lock().unwrap();
            for reader in (*readers).iter() {
                let message = reader.try_recv();
                if let Ok(message) = message {
                    mailbox.send_packet(message.device, message.message);
                }
            }
        }
        drop(trigger);
    });

    rt.block_on(done).unwrap();
}
