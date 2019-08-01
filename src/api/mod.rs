/* Copyright (C) 2017-2019 by Jacob Alexander
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

pub use crate::common_capnp::*;
pub use crate::devicefunction_capnp::*;
pub use crate::hidio_capnp::*;
pub use crate::hidiowatcher_capnp::*;
pub use crate::hostmacro_capnp::*;
pub use crate::usbkeyboard_capnp::*;

use std::cell::RefCell;
use std::collections::HashMap;
use std::io::Write;
use std::net::ToSocketAddrs;
use std::rc::Rc;
use std::sync::atomic::Ordering;
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex, RwLock};

use crate::built_info;
use crate::common_capnp::h_i_d_i_o_node::*;
use crate::device::{HIDIOMailbox, HIDIOMessage};
use crate::hidio_capnp::h_i_d_i_o::*;
use crate::hidio_capnp::h_i_d_i_o_server::*;
use crate::protocol::hidio::HIDIOCommandID;
use crate::RUNNING;
use capnp::capability::Promise;
use capnp::Error;
use capnp_rpc::{pry, rpc_twoparty_capnp, twoparty, RpcSystem};
use lazy_static::lazy_static;
use nanoid;
use rcgen::generate_simple_self_signed;
use stream_cancel::{StreamExt, Tripwire};
use tempfile::NamedTempFile;
use tokio::io::AsyncRead;
use tokio::prelude::*;
use tokio_rustls::{
    rustls::{NoClientAuth, ServerConfig},
    TlsAcceptor,
};
use u_s_b_keyboard::commands::*;

#[cfg(target_family = "unix")]
use std::os::unix::fs::PermissionsExt;

const LISTEN_ADDR: &str = "localhost:7185";

#[cfg(debug_assertions)]
const AUTH_LEVEL: AuthLevel = AuthLevel::Debug;

#[cfg(not(debug_assertions))]
const AUTH_LEVEL: AuthLevel = AuthLevel::Secure;

lazy_static! {
    static ref WRITERS_RC: Arc<Mutex<Vec<std::sync::mpsc::Sender<HIDIOMessage>>>> =
        Arc::new(Mutex::new(vec![]));
    static ref READERS_RC: Arc<Mutex<Vec<std::sync::mpsc::Receiver<HIDIOMessage>>>> =
        Arc::new(Mutex::new(vec![]));
}

// ----- Functions -----

impl std::fmt::Display for NodeType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            NodeType::HidioDaemon => write!(f, "HidioDaemon"),
            NodeType::HidioApi => write!(f, "HidioApi"),
            NodeType::UsbKeyboard => write!(f, "UsbKeyboard"),
        }
    }
}
impl std::fmt::Debug for NodeType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

/// Authorization level for a remote node
#[derive(Clone, Copy)]
pub enum AuthLevel {
    /// Allows connecting and listing devices
    Basic,
    /// Allows sending commands to a device
    Secure,
    /// Allows inspecting all incoming packets
    Debug,
}

/// Information about a connected node
#[derive(Debug, Clone)]
pub struct Endpoint {
    pub type_: NodeType,
    pub name: String,
    pub serial: String,
    /// Automatically generated
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

#[cfg(target_family = "unix")]
pub fn set_file_permissions(file: &mut std::fs::File, restrictive: bool) {
    // Sets the appropriate file permissions
    let mut permissions = file.metadata().unwrap().permissions();
    if restrictive {
        // Restrictive enforces that only this user may read the file
        permissions.set_mode(0o644);
        assert_eq!(permissions.mode(), 0o644);
    } else {
        // R/W for this user, readable by everyone else
        permissions.set_mode(0o600);
        assert_eq!(permissions.mode(), 0o600);
    }
}

#[cfg(target_family = "windows")]
pub fn set_file_permissions(file: &mut std::fs::File, restrictive: bool) {
    // Sets the appropriate file permissions
    // Restrictive enforces that only this user may read the file
    println!("TODO: Windows file permissions not yet complete!");
    if restrictive {
        println!("THIS IS A SERIOUS BUG FOR THE AUTH KEY");
    }
}

struct HIDIOServerImpl {
    master: Rc<RefCell<HIDIOMaster>>,
    uid: u64,
    incoming: Rc<HIDIOMailbox>,

    basic_key: String,
    auth_key: String,

    basic_key_file: NamedTempFile,
    auth_key_file: NamedTempFile,
}

impl HIDIOServerImpl {
    fn new(master: Rc<RefCell<HIDIOMaster>>, uid: u64, incoming: HIDIOMailbox) -> HIDIOServerImpl {
        // Create temp file for basic key
        let mut basic_key_file = NamedTempFile::new().expect("Unable to create file");
        set_file_permissions(basic_key_file.as_file_mut(), false);

        // Create temp file for auth key
        // Only this user can read the auth key
        let mut auth_key_file = NamedTempFile::new().expect("Unable to create file");
        set_file_permissions(auth_key_file.as_file_mut(), true);

        let incoming = Rc::new(incoming);

        // Generate basic and auth keys
        // XXX - Auth key must only be readable by this user
        //       Basic key is world readable
        //       These keys are purposefully not sent over RPC
        //       to enforce local-only connections.
        HIDIOServerImpl {
            master,
            uid,
            incoming,

            basic_key: nanoid::simple().to_string(),
            auth_key: nanoid::simple().to_string(),

            // Set to empty for now
            basic_key_file,
            auth_key_file,
        }
    }

    fn refresh_files(&mut self) {
        // Writes basic key to file
        self.basic_key_file
            .write_all(self.basic_key.as_bytes())
            .expect("Unable to write file");
        set_file_permissions(self.basic_key_file.as_file_mut(), false);

        // Writes auth key to file
        self.auth_key_file
            .write_all(self.auth_key.as_bytes())
            .expect("Unable to write file");
        set_file_permissions(self.auth_key_file.as_file_mut(), true);
    }

    fn create_connection(&mut self, mut node: Endpoint, auth: AuthLevel) -> h_i_d_i_o::Client {
        {
            let mut m = self.master.borrow_mut();
            node.id = self.uid;
            m.connections.get_mut(&self.uid).unwrap().push(node.id);
            m.nodes.push(node);
        }

        info!("Connection authed!");
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
        let key = pry!(pry!(params.get()).get_key());
        let node = Endpoint {
            type_: info.get_type().unwrap(),
            name: info.get_name().unwrap().to_string(),
            serial: info.get_serial().unwrap().to_string(),
            id: info.get_id(),
        };

        // Verify incoming basic key
        if key != self.basic_key {
            return Promise::err(Error {
                kind: capnp::ErrorKind::Failed,
                description: "Authentication denied".to_string(),
            });
        }

        info!("New capnp node: {:?}", node);
        results
            .get()
            .set_port(self.create_connection(node, AuthLevel::Basic));
        Promise::ok(())
    }

    fn auth(&mut self, params: AuthParams, mut results: AuthResults) -> Promise<(), Error> {
        let info = pry!(pry!(params.get()).get_info());
        let key = pry!(pry!(params.get()).get_key());
        let node = Endpoint {
            type_: info.get_type().unwrap(),
            name: info.get_name().unwrap().to_string(),
            serial: info.get_serial().unwrap().to_string(),
            id: info.get_id(),
        };

        // Verify incoming auth key
        if key != self.auth_key {
            return Promise::err(Error {
                kind: capnp::ErrorKind::Failed,
                description: "Authentication denied".to_string(),
            });
        }

        info!("New capnp node: {:?}", node);
        results
            .get()
            .set_port(self.create_connection(node, AUTH_LEVEL));
        Promise::ok(())
    }

    fn version(
        &mut self,
        _params: VersionParams,
        mut results: VersionResults,
    ) -> Promise<(), Error> {
        // Get and set fields
        let mut version = results.get().init_version();
        version.set_version(&format!(
            "{}{}",
            built_info::PKG_VERSION,
            built_info::GIT_VERSION.map_or_else(|| "".to_owned(), |v| format!(" (git {})", v))
        ));
        version.set_buildtime(&built_info::BUILT_TIME_UTC.to_string());
        version.set_serverarch(&built_info::TARGET.to_string());
        version.set_compilerversion(&built_info::RUSTC_VERSION.to_string());
        Promise::ok(())
    }

    fn alive(&mut self, _params: AliveParams, mut results: AliveResults) -> Promise<(), Error> {
        results.get().set_alive(true);
        Promise::ok(())
    }

    fn key(&mut self, _params: KeyParams, mut results: KeyResults) -> Promise<(), Error> {
        // Get and set fields
        let mut key = results.get().init_key();
        key.set_basic_key_path(&self.basic_key_file.path().display().to_string());
        key.set_auth_key_path(&self.auth_key_file.path().display().to_string());
        Promise::ok(())
    }

    fn id(&mut self, _params: IdParams, mut results: IdResults) -> Promise<(), Error> {
        results.get().set_id(self.uid);
        Promise::ok(())
    }
}

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

/// Cap'n'Proto API Initialization
/// Sets up a localhost socket to deal with localhost-only API usages
/// Requires TLS TODO
/// Some API usages may require external authentication to validate trustworthiness
pub fn initialize(mailbox: HIDIOMailbox) {
    info!("Initializing api...");

    let mut core = ::tokio_core::reactor::Core::new().unwrap();
    let handle = core.handle();

    // Open secured capnproto interface
    let addr = LISTEN_ADDR
        .to_socket_addrs()
        .unwrap()
        .next()
        .expect("could not parse address");
    let socket =
        ::tokio_core::net::TcpListener::bind(&addr, &handle).expect("Failed to open socket");
    println!("API: Listening on {}", addr);

    // Generate new self-signed public/private key
    // Private key is not written to disk and generated each time
    let subject_alt_names = vec!["localhost".to_string()];
    let pair = generate_simple_self_signed(subject_alt_names).unwrap();

    let cert = rustls::Certificate(pair.serialize_der().unwrap());
    let pkey = rustls::PrivateKey(pair.serialize_private_key_der());
    let mut config = ServerConfig::new(NoClientAuth::new());
    config.set_single_cert(vec![cert], pkey).unwrap();
    let config = TlsAcceptor::from(Arc::new(config));

    let nodes = mailbox.nodes.clone();
    let m = HIDIOMaster::new(nodes.clone());

    let master = Rc::new(RefCell::new(m));

    let (trigger, tripwire) = Tripwire::new();

    trait Duplex: tokio::io::AsyncRead + tokio::io::AsyncWrite {};
    impl<T> Duplex for T where T: tokio::io::AsyncRead + tokio::io::AsyncWrite {}

    let connections = socket.incoming().take_until(tripwire);
    let tls_handshake = connections.map(|(socket, addr)| {
        info!("New connection:7185 - {:?}", addr);
        socket.set_nodelay(true).unwrap();
        config.accept(socket)
    });

    let server = tls_handshake.map(|acceptor| {
        let rc = Rc::clone(&master);
        let (hidapi_writer, hidapi_reader) = channel::<HIDIOMessage>();
        let (sink, mailbox) = HIDIOMailbox::from_sender(hidapi_writer.clone(), nodes.clone());
        {
            let mut writers = WRITERS_RC.lock().unwrap();
            (*writers).push(sink);
            let mut readers = READERS_RC.lock().unwrap();
            (*readers).push(hidapi_reader);
        }

        let handle = handle.clone();
        acceptor.and_then(move |stream| {
            // Save connection address for later
            let addr = stream.get_ref().0.peer_addr().ok().unwrap();

            // Assign a uid to the connection
            let uid = {
                let mut m = rc.borrow_mut();
                m.last_uid += 1;
                let uid = m.last_uid;
                m.connections.insert(uid, vec![]);
                uid
            };

            // Initialize auth tokens
            let mut hidio_server = HIDIOServerImpl::new(Rc::clone(&rc), uid, mailbox);
            hidio_server.refresh_files();

            // Setup capnproto server
            let hidio_server =
                h_i_d_i_o_server::ToClient::new(hidio_server).into_client::<::capnp_rpc::Server>();

            let (reader, writer) = stream.split();
            let network = twoparty::VatNetwork::new(
                reader,
                writer,
                rpc_twoparty_capnp::Side::Server,
                Default::default(),
            );

            // Setup capnproto RPC
            let rpc_system = RpcSystem::new(Box::new(network), Some(hidio_server.clone().client));
            handle.spawn(
                rpc_system
                    .map_err(|e| println!("{}", e))
                    .and_then(move |_| {
                        info!("Connection closed:7185 - {:?}", addr);

                        // Client disconnected, delete node
                        let connected_nodes = &rc.borrow().connections[&uid].clone();
                        rc.borrow_mut()
                            .nodes
                            .retain(|x| !connected_nodes.contains(&x.id));
                        Ok(())
                    }),
            );
            Ok(())
        })
    });

    // Mailbox thread
    std::thread::spawn(move || {
        loop {
            if !RUNNING.load(Ordering::SeqCst) {
                break;
            }
            let message = mailbox.recv_psuedoblocking();
            if let Some(message) = message {
                let mut writers = WRITERS_RC.lock().unwrap();
                *writers = (*writers)
                    .drain_filter(|writer| writer.send(message.clone()).is_ok())
                    .collect::<Vec<_>>();
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

    core.run(server.for_each(|client| {
        handle.spawn(client.map_err(|e| println!("{}", e)));
        Ok(())
    }))
    .unwrap();
}
