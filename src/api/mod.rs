/* Copyright (C) 2017-2020 by Jacob Alexander
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
use std::env;
use std::io::Write;
use std::net::ToSocketAddrs;
use std::rc::Rc;
use std::sync::atomic::Ordering;
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Instant;

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
use glob::glob;
use lazy_static::lazy_static;
use rcgen::generate_simple_self_signed;
use stream_cancel::{StreamExt, Tripwire};
use tokio::io::AsyncRead;
use tokio::prelude::*;
use tokio_rustls::{
    rustls::{NoClientAuth, ServerConfig},
    TlsAcceptor,
};
use u_s_b_keyboard::commands::*;

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
            NodeType::BleKeyboard => write!(f, "BleKeyboard"),
        }
    }
}
impl std::fmt::Debug for NodeType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

/// Authorization level for a remote node
#[derive(Clone, Copy, Debug)]
pub enum AuthLevel {
    /// Allows connecting and listing devices
    Basic,
    /// Allows sending commands to a device
    Secure,
    /// Allows inspecting all incoming packets
    Debug,
}

/// HIDAPI Information
#[derive(Debug, Clone, Default)]
pub struct HIDAPIInfo {
    pub path: String,
    pub vendor_id: u16,
    pub product_id: u16,
    pub serial_number: String,
    pub release_number: u16,
    pub manufacturer_string: String,
    pub product_string: String,
    pub usage_page: u16,
    pub usage: u16,
    pub interface_number: i32,
}

impl HIDAPIInfo {
    pub fn build_hidapi_key(&mut self) -> String {
        format!(
            "vid:{:04x} pid:{:04x} serial:{} manufacturer:{} product:{} usage_page:{:x} usage:{:x} interface:{}",
            self.vendor_id,
            self.product_id,
            self.serial_number,
            self.manufacturer_string,
            self.product_string,
            self.usage_page,
            self.usage,
            self.interface_number,
        )
    }

    pub fn new(device_info: &hidapi::DeviceInfo) -> HIDAPIInfo {
        HIDAPIInfo {
            path: format!("{:#?}", device_info.path()),
            vendor_id: device_info.vendor_id(),
            product_id: device_info.product_id(),
            serial_number: match device_info.serial_number() {
                Some(s) => s.to_string(),
                _ => "<Serial Unset>".to_string(),
            },
            release_number: device_info.release_number(),
            manufacturer_string: match device_info.manufacturer_string() {
                Some(s) => s.to_string(),
                _ => "<Manufacturer Unset>".to_string(),
            },
            product_string: match device_info.product_string() {
                Some(s) => s.to_string(),
                _ => "<Product Unset>".to_string(),
            },
            usage_page: device_info.usage_page(),
            usage: device_info.usage(),
            interface_number: device_info.interface_number(),
        }
    }
}

/// Information about a connected node
#[derive(Debug, Clone)]
pub struct Endpoint {
    type_: NodeType,
    name: String,   // Used for hidio (e.g. hidioDaemon, hidioApi) types
    serial: String, // Used for hidio (e.g. hidioDaemon, hidioApi) types
    id: u64,
    created: Instant,
    hidapi: HIDAPIInfo,
}

impl std::fmt::Display for Endpoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(
            format!(
                "id:{} {} {}",
                self.id,
                match self.type_ {
                    NodeType::BleKeyboard => format!(
                        "BLE [{:04x}:{:04x}-{:x}:{:x}] {}",
                        self.hidapi.vendor_id,
                        self.hidapi.product_id,
                        self.hidapi.usage_page,
                        self.hidapi.usage,
                        self.hidapi.product_string,
                    ),
                    NodeType::UsbKeyboard => format!(
                        "USB [{:04x}:{:04x}-{:x}:{:x}] [{}] {}",
                        self.hidapi.vendor_id,
                        self.hidapi.product_id,
                        self.hidapi.usage_page,
                        self.hidapi.usage,
                        self.hidapi.manufacturer_string,
                        self.hidapi.product_string,
                    ),
                    _ => self.name.clone(),
                },
                match self.type_ {
                    NodeType::BleKeyboard | NodeType::UsbKeyboard =>
                        self.hidapi.serial_number.clone(),
                    _ => self.serial.clone(),
                },
            )
            .as_str(),
        )
    }
}

impl Endpoint {
    pub fn new(type_: NodeType, id: u64) -> Endpoint {
        Endpoint {
            type_,
            name: "".to_string(),
            serial: "".to_string(),
            id,
            created: Instant::now(),
            hidapi: HIDAPIInfo {
                ..Default::default()
            },
        }
    }

    pub fn set_hidio_params(&mut self, name: String, serial: String) {
        self.name = name;
        self.serial = serial;
    }

    pub fn set_hidapi_params(&mut self, info: HIDAPIInfo) {
        self.hidapi = info;
        self.name = self.name();
        self.serial = self.serial();
    }

    pub fn set_hidapi_path(&mut self, path: String) {
        self.hidapi.path = path;
    }

    pub fn type_(&mut self) -> NodeType {
        self.type_
    }

    pub fn name(&mut self) -> String {
        match self.type_ {
            NodeType::BleKeyboard => format!(
                "[{:04x}:{:04x}-{:x}:{:x}] {}",
                self.hidapi.vendor_id,
                self.hidapi.product_id,
                self.hidapi.usage_page,
                self.hidapi.usage,
                self.hidapi.product_string,
            ),
            NodeType::UsbKeyboard => format!(
                "[{:04x}:{:04x}-{:x}:{:x}] [{}] {}",
                self.hidapi.vendor_id,
                self.hidapi.product_id,
                self.hidapi.usage_page,
                self.hidapi.usage,
                self.hidapi.manufacturer_string,
                self.hidapi.product_string,
            ),
            _ => self.name.clone(),
        }
    }

    /// Used to generate a unique key that will point to this device
    /// Empty fields are still used (in the case of bluetooth and the interface field on Windows
    /// sometimes)
    /// Does not include path, as the path may not uniquely identify device port or device
    /// Does not include release number as this may be incrementing
    pub fn key(&mut self) -> String {
        match self.type_ {
            NodeType::BleKeyboard | NodeType::UsbKeyboard => self.hidapi.build_hidapi_key(),
            _ => format!("name:{} serial:{}", self.name, self.serial,),
        }
    }

    pub fn serial(&mut self) -> String {
        match self.type_ {
            NodeType::BleKeyboard | NodeType::UsbKeyboard => self.hidapi.serial_number.clone(),
            _ => self.serial.clone(),
        }
    }

    pub fn id(&mut self) -> u64 {
        self.id
    }

    pub fn created(&mut self) -> Instant {
        self.created
    }

    pub fn path(&mut self) -> String {
        self.hidapi.path.clone()
    }
}

struct HIDIOMaster {
    nodes: Vec<Endpoint>,
    devices: Arc<RwLock<Vec<Endpoint>>>,
    connections: HashMap<u64, Vec<u64>>,
}

impl HIDIOMaster {
    fn new(devices: Arc<RwLock<Vec<Endpoint>>>) -> HIDIOMaster {
        HIDIOMaster {
            nodes: Vec::new(),
            devices,
            connections: HashMap::new(),
        }
    }
}

struct HIDIOServerImpl {
    master: Rc<RefCell<HIDIOMaster>>,
    uid: u64,
    incoming: Rc<HIDIOMailbox>,

    basic_key: String,
    auth_key: String,

    basic_key_file: tempfile::NamedTempFile,
    auth_key_file: tempfile::NamedTempFile,

    subscribers_next_id: Arc<RwLock<u64>>,
    subscribers: Arc<RwLock<SubscriberMap>>,
}

impl HIDIOServerImpl {
    fn new(
        master: Rc<RefCell<HIDIOMaster>>,
        uid: u64,
        incoming: HIDIOMailbox,
        subscribers_next_id: Arc<RwLock<u64>>,
        subscribers: Arc<RwLock<SubscriberMap>>,
    ) -> HIDIOServerImpl {
        // Create temp file for basic key
        let mut basic_key_file = tempfile::Builder::new()
            .world_accessible(true)
            .tempfile()
            .expect("Unable to create file");

        // Create temp file for auth key
        // Only this user can read the auth key
        let mut auth_key_file = tempfile::Builder::new()
            .world_accessible(false)
            .tempfile()
            .expect("Unable to create file");

        // Generate keys
        let basic_key = nanoid::simple();
        let auth_key = nanoid::simple();

        // Writes basic key to file
        basic_key_file
            .write_all(basic_key.as_bytes())
            .expect("Unable to write file");

        // Writes auth key to file
        auth_key_file
            .write_all(auth_key.as_bytes())
            .expect("Unable to write file");

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

            basic_key,
            auth_key,

            basic_key_file,
            auth_key_file,

            subscribers_next_id,
            subscribers,
        }
    }

    fn create_connection(&mut self, mut node: Endpoint, auth: AuthLevel) -> h_i_d_i_o::Client {
        {
            let mut m = self.master.borrow_mut();
            node.id = self.uid;
            let conn = m.connections.get_mut(&self.uid).unwrap();
            // Check if a capnp node already exists (might just be re-authenticating the interface)
            if !conn.contains(&node.id) {
                info!("New capnp node: {:?}", node);
                conn.push(node.id);
                m.nodes.push(node);
            }
        }

        info!("Connection authed! - {:?}", auth);
        h_i_d_i_o::ToClient::new(HIDIOImpl::new(
            Rc::clone(&self.master),
            Rc::clone(&self.incoming),
            auth,
            self.subscribers_next_id.clone(),
            self.subscribers.clone(),
        ))
        .into_client::<::capnp_rpc::Server>()
    }
}

impl h_i_d_i_o_server::Server for HIDIOServerImpl {
    fn basic(&mut self, params: BasicParams, mut results: BasicResults) -> Promise<(), Error> {
        let info = pry!(pry!(params.get()).get_info());
        let key = pry!(pry!(params.get()).get_key());
        let mut node = Endpoint::new(info.get_type().unwrap(), info.get_id());
        node.set_hidio_params(
            info.get_name().unwrap().to_string(),
            info.get_serial().unwrap().to_string(),
        );

        // Verify incoming basic key
        if key != self.basic_key {
            return Promise::err(Error {
                kind: capnp::ErrorKind::Failed,
                description: "Authentication denied".to_string(),
            });
        }

        // Either re-use a capnp node or create a new one
        results
            .get()
            .set_port(self.create_connection(node, AuthLevel::Basic));
        Promise::ok(())
    }

    fn auth(&mut self, params: AuthParams, mut results: AuthResults) -> Promise<(), Error> {
        let info = pry!(pry!(params.get()).get_info());
        let key = pry!(pry!(params.get()).get_key());
        let mut node = Endpoint::new(info.get_type().unwrap(), info.get_id());
        node.set_hidio_params(
            info.get_name().unwrap().to_string(),
            info.get_serial().unwrap().to_string(),
        );

        // Verify incoming auth key
        if key != self.auth_key {
            return Promise::err(Error {
                kind: capnp::ErrorKind::Failed,
                description: "Authentication denied".to_string(),
            });
        }

        // Either re-use a capnp node or create a new one
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

    fn name(&mut self, _params: NameParams, mut results: NameResults) -> Promise<(), Error> {
        results.get().set_name("hid-io-core");
        Promise::ok(())
    }

    fn log_files(
        &mut self,
        _params: LogFilesParams,
        mut results: LogFilesResults,
    ) -> Promise<(), Error> {
        // Get list of log files
        let path = env::temp_dir()
            .join("hid-io-core*.log")
            .into_os_string()
            .into_string()
            .unwrap();
        let files: Vec<_> = glob(path.as_str())
            .expect("Failed to find log files...")
            .collect();
        let mut result = results.get().init_paths(files.len() as u32);
        for (i, f) in files.iter().enumerate() {
            if let Ok(f) = f {
                result.set(
                    i as u32,
                    f.clone().into_os_string().into_string().unwrap().as_str(),
                );
            }
        }
        Promise::ok(())
    }
}

struct HIDIOImpl {
    master: Rc<RefCell<HIDIOMaster>>,
    auth: AuthLevel,
    registered: Rc<RefCell<HashMap<u64, bool>>>,
    incoming: Rc<HIDIOMailbox>,

    nodes_subscribers_next_id: Arc<RwLock<u64>>,
    nodes_subscribers: Arc<RwLock<SubscriberMap>>,
}

impl HIDIOImpl {
    fn new(
        master: Rc<RefCell<HIDIOMaster>>,
        incoming: Rc<HIDIOMailbox>,
        auth: AuthLevel,
        nodes_subscribers_next_id: Arc<RwLock<u64>>,
        nodes_subscribers: Arc<RwLock<SubscriberMap>>,
    ) -> HIDIOImpl {
        HIDIOImpl {
            master,
            auth,
            registered: Rc::new(RefCell::new(HashMap::new())),
            incoming,

            nodes_subscribers_next_id,
            nodes_subscribers,
        }
    }

    fn init_signal(
        &self,
        mut signal: h_i_d_i_o::signal::Builder<'_>,
        message: HIDIOMessage,
    ) -> Promise<(), Error> {
        signal.set_time(15);

        {
            let master = self.master.borrow();
            let devices = &master.devices.read().unwrap();
            let device = match devices.iter().find(|d| d.id.to_string() == message.device) {
                None => {
                    return Promise::err(capnp::Error {
                        kind: capnp::ErrorKind::Failed,
                        description: format!("Could not find id {} in device list", message.device),
                    })
                }
                Some(v) => v,
            };

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
        Promise::ok(())
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
                    self.init_signal(signal, message)
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

    fn subscribe_nodes(
        &mut self,
        params: SubscribeNodesParams,
        mut results: SubscribeNodesResults,
    ) -> Promise<(), Error> {
        info!(
            "Adding subscribeNodes watcher id #{}",
            self.nodes_subscribers_next_id.read().unwrap()
        );
        self.nodes_subscribers.write().unwrap().subscribers.insert(
            *self.nodes_subscribers_next_id.read().unwrap(),
            SubscriberHandle {
                client: pry!(pry!(params.get()).get_subscriber()),
                requests_in_flight: 0,
            },
        );

        results.get().set_subscription(
            nodes_subscription::ToClient::new(NodesSubscriptionImpl::new(
                *self.nodes_subscribers_next_id.read().unwrap(),
                self.nodes_subscribers.clone(),
            ))
            .into_client::<::capnp_rpc::Server>(),
        );

        *self.nodes_subscribers_next_id.write().unwrap() += 1;
        Promise::ok(())
    }
}

struct SubscriberHandle {
    client: nodes_subscriber::Client,
    requests_in_flight: i32,
}

struct SubscriberMap {
    subscribers: HashMap<u64, SubscriberHandle>,
}

impl SubscriberMap {
    fn new() -> SubscriberMap {
        SubscriberMap {
            subscribers: HashMap::new(),
        }
    }
}

struct NodesSubscriptionImpl {
    id: u64,
    subscribers: Arc<RwLock<SubscriberMap>>,
}

impl NodesSubscriptionImpl {
    fn new(id: u64, subscribers: Arc<RwLock<SubscriberMap>>) -> NodesSubscriptionImpl {
        NodesSubscriptionImpl { id, subscribers }
    }
}

impl Drop for NodesSubscriptionImpl {
    fn drop(&mut self) {
        info!("Subscription dropped id: {}", self.id);
        self.subscribers
            .write()
            .unwrap()
            .subscribers
            .remove(&self.id);
    }
}

impl nodes_subscription::Server for NodesSubscriptionImpl {}

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
    let last_uid = mailbox.last_uid.clone();

    let master = Rc::new(RefCell::new(m));

    let subscribers_next_id = Arc::new(RwLock::new(0));
    let subscribers = Arc::new(RwLock::new(SubscriberMap::new()));

    let (trigger, tripwire) = Tripwire::new();

    trait Duplex: tokio::io::AsyncRead + tokio::io::AsyncWrite {};
    impl<T> Duplex for T where T: tokio::io::AsyncRead + tokio::io::AsyncWrite {}

    let connections = socket.incoming().take_until(tripwire.clone());
    let tls_handshake = connections.map(|(socket, addr)| {
        info!("New connection:7185 - {:?}", addr);
        socket.set_nodelay(true).unwrap();
        config.accept(socket)
    });

    let server =
        tls_handshake.map(|acceptor| {
            let rc = Rc::clone(&master);
            let last_uid = last_uid.clone();
            let (hidapi_writer, hidapi_reader) = channel::<HIDIOMessage>();
            let (sink, mailbox) =
                HIDIOMailbox::from_sender(hidapi_writer, nodes.clone(), last_uid.clone());
            {
                let mut writers = WRITERS_RC.lock().unwrap();
                (*writers).push(sink);
                let mut readers = READERS_RC.lock().unwrap();
                (*readers).push(hidapi_reader);
            }

            let handle = handle.clone();
            let subscribers_next_id = subscribers_next_id.clone();
            let subscribers = subscribers.clone();
            acceptor.and_then(move |stream| {
                // Save connection address for later
                let addr = stream.get_ref().0.peer_addr().ok().unwrap();

                // Assign a uid to the connection
                let uid = {
                    let mut m = rc.borrow_mut();
                    // Increment
                    (*last_uid.write().unwrap()) += 1;
                    let this_uid = *last_uid.read().unwrap();
                    m.connections.insert(this_uid, vec![]);
                    this_uid
                };

                // Initialize auth tokens
                let hidio_server = HIDIOServerImpl::new(
                    Rc::clone(&rc),
                    uid,
                    mailbox,
                    subscribers_next_id,
                    subscribers,
                );

                // Setup capnproto server
                let hidio_server = h_i_d_i_o_server::ToClient::new(hidio_server)
                    .into_client::<::capnp_rpc::Server>();

                let (reader, writer) = stream.split();
                let network = twoparty::VatNetwork::new(
                    reader,
                    writer,
                    rpc_twoparty_capnp::Side::Server,
                    Default::default(),
                );

                // Setup capnproto RPC
                let rpc_system = RpcSystem::new(Box::new(network), Some(hidio_server.client));
                handle.spawn(rpc_system.map_err(|e| info!("rpc_system: {}", e)).and_then(
                    move |_| {
                        info!("Connection closed:7185 - {:?} - uid:{}", addr, uid);

                        // Client disconnected, delete node
                        let connected_nodes = &rc.borrow().connections[&uid].clone();
                        rc.borrow_mut()
                            .nodes
                            .retain(|x| !connected_nodes.contains(&x.id));
                        Ok(())
                    },
                ));
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

    let infinite = ::futures::stream::iter_ok::<_, std::io::Error>(::std::iter::repeat(()));
    let rc = Rc::clone(&master);
    let mut last_node_refresh = Instant::now();
    let mut last_node_count = 0;
    let send_to_subscribers = infinite.take_until(tripwire).fold(
        (handle.clone(), subscribers.clone()),
        move |(handle, subscribers),
              ()|
              -> Promise<
            (::tokio_core::reactor::Handle, Arc<RwLock<SubscriberMap>>),
            std::io::Error,
        > {
            {
                let sub_count = subscribers.read().unwrap().subscribers.len();
                let subs = &mut subscribers.write().unwrap().subscribers;
                let subscribers1 = subscribers.clone();

                let master = rc.borrow();

                // Determine most recent device addition
                let devices = master.devices.read().unwrap();
                let nodes = master.nodes.clone();
                let mut nodes_update = false;
                let mut cur_node_count = 0;
                nodes.iter().chain(devices.iter()).for_each(|endpoint| {
                    if let Some(_duration) =
                        endpoint.created.checked_duration_since(last_node_refresh)
                    {
                        nodes_update = true;
                    }
                    // Count total nodes, if total count doesn't match the last loop
                    // a nodes update should be sent (node removal case)
                    cur_node_count += 1;
                });
                if cur_node_count != last_node_count {
                    nodes_update = true;
                }
                last_node_count = cur_node_count;

                if nodes_update {
                    info!(
                        "Node list update detected, pushing list to subscribers -> {}",
                        sub_count
                    );

                    for (&idx, mut subscriber) in subs.iter_mut() {
                        if subscriber.requests_in_flight < 5 {
                            subscriber.requests_in_flight += 1;
                            let mut request = subscriber.client.nodes_update_request();
                            {
                                let mut c_nodes = request.get().init_nodes(last_node_count as u32);
                                for (i, n) in nodes.iter().chain(devices.iter()).enumerate() {
                                    let mut node = c_nodes.reborrow().get(i as u32);
                                    node.set_type(n.type_);
                                    node.set_name(&n.name);
                                    node.set_serial(&n.serial);
                                    node.set_id(n.id);
                                    /* TODO(HaaTa): This field may be tricky to get
                                     *              Use nodes() rpc call for now
                                    node.set_node(
                                        h_i_d_i_o_node::ToClient::new(HIDIONodeImpl::new(
                                            Rc::clone(&self.registered),
                                            n.id,
                                        ))
                                        .into_client::<::capnp_rpc::Server>(),
                                    );
                                    */
                                    /* TODO(HaaTa): This field may be tricky to get
                                     *              Use nodes() rpc call for now
                                    commands.set_usb_keyboard(
                                        u_s_b_keyboard::commands::ToClient::new(HIDIOKeyboardNodeImpl::new(
                                            n.id,
                                            self.auth,
                                            Rc::clone(&self.incoming),
                                        ))
                                        .into_client::<::capnp_rpc::Server>(),
                                    );
                                    */
                                }
                            }

                            //request.get().set_nodes();
                            //pry!(request.get().set_message(
                            //    &format!("system time is: {:?}", ::std::time::SystemTime::now())[..]));
                            //request.get().set_message(&format!("YARRR {}", sub_count));

                            let subscribers2 = subscribers1.clone();
                            handle.spawn(
                                request
                                    .send()
                                    .promise
                                    .then(move |r| {
                                        match r {
                                            Ok(_) => {
                                                if let Some(ref mut s) = subscribers2
                                                    .write()
                                                    .unwrap()
                                                    .subscribers
                                                    .get_mut(&idx)
                                                {
                                                    s.requests_in_flight -= 1;
                                                }
                                            }
                                            Err(e) => {
                                                warn!("Got error: {:?}. Dropping subscriber.", e);
                                                subscribers2
                                                    .write()
                                                    .unwrap()
                                                    .subscribers
                                                    .remove(&idx);
                                            }
                                        }
                                        Ok::<(), std::io::Error>(())
                                    })
                                    .map_err(|_| unreachable!()),
                            );
                        }
                    }
                    last_node_refresh = Instant::now();
                }
            }
            let timeout = pry!(::tokio_core::reactor::Timeout::new(
                ::std::time::Duration::from_secs(1),
                &handle
            ));
            let timeout = timeout
                .and_then(move |()| Ok((handle, subscribers)))
                .map_err(|_| unreachable!());
            Promise::from_future(timeout)
        },
    );

    core.run(
        server
            .for_each(|client| {
                handle.spawn(client.map_err(|e| info!("core.run.server: {}", e)));
                Ok(())
            })
            .join(send_to_subscribers),
    )
    .unwrap();
}
