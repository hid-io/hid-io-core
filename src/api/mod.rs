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

pub use crate::common_capnp;
pub use crate::daemon_capnp;
pub use crate::hidio_capnp;
pub use crate::keyboard_capnp;

use crate::built_info;
use crate::mailbox;
use crate::protocol::hidio::HIDIOCommandID;
use crate::RUNNING;
use capnp::capability::Promise;
use capnp::Error;
use capnp_rpc::{pry, rpc_twoparty_capnp, twoparty, RpcSystem};
use futures::{FutureExt, TryFutureExt};
use glob::glob;
use rcgen::generate_simple_self_signed;
use std::collections::HashMap;
use std::env;
use std::io::Write;
use std::net::ToSocketAddrs;
use std::sync::atomic::Ordering;
use std::sync::{Arc, RwLock};
use std::time::Instant;
use tokio::stream::StreamExt;
use tokio_rustls::{
    rustls::{Certificate, NoClientAuth, PrivateKey, ServerConfig},
    TlsAcceptor,
};

const LISTEN_ADDR: &str = "localhost:7185";

#[cfg(debug_assertions)]
const AUTH_LEVEL: AuthLevel = AuthLevel::Debug;

#[cfg(not(debug_assertions))]
const AUTH_LEVEL: AuthLevel = AuthLevel::Secure;

// ----- Functions -----

impl std::fmt::Display for common_capnp::NodeType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            common_capnp::NodeType::HidioDaemon => write!(f, "HidioDaemon"),
            common_capnp::NodeType::HidioApi => write!(f, "HidioApi"),
            common_capnp::NodeType::UsbKeyboard => write!(f, "UsbKeyboard"),
            common_capnp::NodeType::BleKeyboard => write!(f, "BleKeyboard"),
            common_capnp::NodeType::HidKeyboard => write!(f, "HidKeyboard"),
            common_capnp::NodeType::HidMouse => write!(f, "HidMouse"),
            common_capnp::NodeType::HidJoystick => write!(f, "HidJoystick"),
        }
    }
}
impl std::fmt::Debug for common_capnp::NodeType {
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

/// Uhid Information
/// This is only used on Linux
#[derive(Debug, Clone, Default)]
pub struct UhidInfo {
    // These fields are from uhid_virt::CreateParams
    pub name: String,
    pub phys: String,
    pub uniq: String,
    pub bus: u16,
    pub vendor: u32,
    pub product: u32,
    pub version: u32,
    pub country: u32,
}

impl UhidInfo {
    /// Generate a unique string based off of evdev information (excluding path/physical location)
    pub fn key(&mut self) -> String {
        format!(
            "vendor:{:04x} product:{:04x} name:{} phys:{} uniq:{} bus:{} version:{} country:{}",
            self.vendor,
            self.product,
            self.name,
            self.phys,
            self.uniq,
            self.bus,
            self.version,
            self.country,
        )
    }

    pub fn new(params: uhid_virt::CreateParams) -> UhidInfo {
        UhidInfo {
            name: params.name,
            phys: params.phys,
            uniq: params.uniq,
            bus: params.bus as u16,
            vendor: params.vendor,
            product: params.product,
            version: params.version,
            country: params.country,
        }
    }
}

/// Evdev Information
/// This is only used on Linux
#[derive(Debug, Clone, Default)]
pub struct EvdevInfo {
    // These fields are from evdev_rs::Device
    pub name: String,
    pub phys: String,
    pub uniq: String,
    pub product_id: u16,
    pub vendor_id: u16,
    pub bustype: u16,
    pub version: u16,
    pub driver_version: i32,
}

impl EvdevInfo {
    /// Generate a unique string based off of evdev information (excluding path/physical location)
    pub fn key(&mut self) -> String {
        format!(
            "vid:{:04x} pid:{:04x} name:{} phys:{} uniq:{} bus:{} version:{} driver_version:{}",
            self.vendor_id,
            self.product_id,
            self.name,
            self.phys,
            self.uniq,
            self.bustype,
            self.version,
            self.driver_version,
        )
    }

    pub fn new(device: evdev_rs::Device) -> EvdevInfo {
        EvdevInfo {
            name: device.name().unwrap_or("").to_string(),
            phys: device.phys().unwrap_or("").to_string(),
            uniq: device.uniq().unwrap_or("").to_string(),
            product_id: device.product_id(),
            vendor_id: device.vendor_id(),
            bustype: device.bustype(),
            version: device.version(),
            driver_version: device.driver_version(),
        }
    }
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
    /// Generate a unique string based off of hidapi information (excluding path/physical location)
    pub fn key(&mut self) -> String {
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
    type_: common_capnp::NodeType,
    name: String,   // Used for hidio (e.g. hidioDaemon, hidioApi) types
    serial: String, // Used for hidio (e.g. hidioDaemon, hidioApi) types
    uid: u64,
    created: Instant,
    hidapi: HIDAPIInfo,
    evdev: EvdevInfo,
    uhid: UhidInfo,
}

impl std::fmt::Display for Endpoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(
            format!(
                "id:{} {} {}",
                self.uid,
                match self.type_ {
                    common_capnp::NodeType::BleKeyboard => format!(
                        "BLE [{:04x}:{:04x}-{:x}:{:x}] {}",
                        self.hidapi.vendor_id,
                        self.hidapi.product_id,
                        self.hidapi.usage_page,
                        self.hidapi.usage,
                        self.hidapi.product_string,
                    ),
                    common_capnp::NodeType::UsbKeyboard => format!(
                        "USB [{:04x}:{:04x}-{:x}:{:x}] [{}] {}",
                        self.hidapi.vendor_id,
                        self.hidapi.product_id,
                        self.hidapi.usage_page,
                        self.hidapi.usage,
                        self.hidapi.manufacturer_string,
                        self.hidapi.product_string,
                    ),
                    // TODO Display Hid devices, but handle in a cross-platform way
                    _ => self.name.clone(),
                },
                match self.type_ {
                    common_capnp::NodeType::BleKeyboard | common_capnp::NodeType::UsbKeyboard =>
                        self.hidapi.serial_number.clone(),
                    _ => self.serial.clone(),
                },
            )
            .as_str(),
        )
    }
}

impl Endpoint {
    pub fn new(type_: common_capnp::NodeType, uid: u64) -> Endpoint {
        Endpoint {
            type_,
            name: "".to_string(),
            serial: "".to_string(),
            uid,
            created: Instant::now(),
            hidapi: HIDAPIInfo {
                ..Default::default()
            },
            evdev: EvdevInfo {
                ..Default::default()
            },
            uhid: UhidInfo {
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

    pub fn set_evdev_params(&mut self, info: EvdevInfo) {
        self.evdev = info;
        self.name = self.name();
        self.serial = self.serial();
    }

    pub fn set_uhid_params(&mut self, info: UhidInfo) {
        self.uhid = info;
        self.name = self.name();
        self.serial = self.serial();
    }

    pub fn set_hidapi_path(&mut self, path: String) {
        self.hidapi.path = path;
    }

    pub fn type_(&mut self) -> common_capnp::NodeType {
        self.type_
    }

    pub fn name(&mut self) -> String {
        match self.type_ {
            common_capnp::NodeType::BleKeyboard => format!(
                "[{:04x}:{:04x}-{:x}:{:x}] {}",
                self.hidapi.vendor_id,
                self.hidapi.product_id,
                self.hidapi.usage_page,
                self.hidapi.usage,
                self.hidapi.product_string,
            ),
            common_capnp::NodeType::UsbKeyboard => format!(
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
            common_capnp::NodeType::BleKeyboard | common_capnp::NodeType::UsbKeyboard => {
                self.hidapi.key()
            }
            _ => format!("name:{} serial:{}", self.name, self.serial,),
        }
    }

    pub fn serial(&mut self) -> String {
        match self.type_ {
            common_capnp::NodeType::BleKeyboard | common_capnp::NodeType::UsbKeyboard => {
                self.hidapi.serial_number.clone()
            }
            _ => self.serial.clone(),
        }
    }

    pub fn uid(&mut self) -> u64 {
        self.uid
    }

    pub fn created(&mut self) -> Instant {
        self.created
    }

    pub fn path(&mut self) -> String {
        self.hidapi.path.clone()
    }
}

struct Subscriptions {
    // Node list subscriptions
    nodes_next_id: u64,
    nodes: NodesSubscriberMap,

    // HIDIO Keyboard node subscriptions
    keyboard_node_next_id: u64,
    keyboard_node: KeyboardSubscriberMap,

    // HIDIO Daemon node subscriptions
    daemon_node_next_id: u64,
    daemon_node: DaemonSubscriberMap,
}

impl Subscriptions {
    fn new() -> Subscriptions {
        Subscriptions {
            nodes_next_id: 0,
            nodes: NodesSubscriberMap::new(),
            keyboard_node_next_id: 0,
            keyboard_node: KeyboardSubscriberMap::new(),
            daemon_node_next_id: 0,
            daemon_node: DaemonSubscriberMap::new(),
        }
    }
}

struct HIDIOServerImpl {
    mailbox: mailbox::Mailbox,
    connections: Arc<RwLock<HashMap<u64, Vec<u64>>>>,
    uid: u64,

    basic_key: String,
    auth_key: String,

    basic_key_file: tempfile::NamedTempFile,
    auth_key_file: tempfile::NamedTempFile,

    subscriptions: Arc<RwLock<Subscriptions>>,
}

impl HIDIOServerImpl {
    fn new(
        mailbox: mailbox::Mailbox,
        connections: Arc<RwLock<HashMap<u64, Vec<u64>>>>,
        uid: u64,
        subscriptions: Arc<RwLock<Subscriptions>>,
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

        // Generate basic and auth keys
        // XXX - Auth key must only be readable by this user
        //       Basic key is world readable
        //       These keys are purposefully not sent over RPC
        //       to enforce local-only connections.
        HIDIOServerImpl {
            mailbox,
            connections,
            uid,

            basic_key,
            auth_key,

            basic_key_file,
            auth_key_file,

            subscriptions,
        }
    }

    fn create_connection(
        &mut self,
        mut node: Endpoint,
        auth: AuthLevel,
    ) -> hidio_capnp::h_i_d_i_o::Client {
        {
            let mut connections = self.connections.write().unwrap();
            node.uid = self.uid;
            let conn = connections.get_mut(&self.uid).unwrap();
            // Check if a capnp node already exists (might just be re-authenticating the interface)
            if !conn.contains(&node.uid) {
                info!("New capnp node: {:?}", node);
                conn.push(node.uid);
                self.mailbox.nodes.write().unwrap().push(node.clone());
            }
        }

        info!("Connection authed! - {:?}", auth);
        capnp_rpc::new_client(HIDIOImpl::new(
            self.mailbox.clone(),
            node,
            auth,
            self.subscriptions.clone(),
        ))
    }
}

impl hidio_capnp::h_i_d_i_o_server::Server for HIDIOServerImpl {
    fn basic(
        &mut self,
        params: hidio_capnp::h_i_d_i_o_server::BasicParams,
        mut results: hidio_capnp::h_i_d_i_o_server::BasicResults,
    ) -> Promise<(), Error> {
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

    fn auth(
        &mut self,
        params: hidio_capnp::h_i_d_i_o_server::AuthParams,
        mut results: hidio_capnp::h_i_d_i_o_server::AuthResults,
    ) -> Promise<(), Error> {
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
        _params: hidio_capnp::h_i_d_i_o_server::VersionParams,
        mut results: hidio_capnp::h_i_d_i_o_server::VersionResults,
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

    fn alive(
        &mut self,
        _params: hidio_capnp::h_i_d_i_o_server::AliveParams,
        mut results: hidio_capnp::h_i_d_i_o_server::AliveResults,
    ) -> Promise<(), Error> {
        results.get().set_alive(true);
        Promise::ok(())
    }

    fn key(
        &mut self,
        _params: hidio_capnp::h_i_d_i_o_server::KeyParams,
        mut results: hidio_capnp::h_i_d_i_o_server::KeyResults,
    ) -> Promise<(), Error> {
        // Get and set fields
        let mut key = results.get().init_key();
        key.set_basic_key_path(&self.basic_key_file.path().display().to_string());
        key.set_auth_key_path(&self.auth_key_file.path().display().to_string());
        Promise::ok(())
    }

    fn id(
        &mut self,
        _params: hidio_capnp::h_i_d_i_o_server::IdParams,
        mut results: hidio_capnp::h_i_d_i_o_server::IdResults,
    ) -> Promise<(), Error> {
        results.get().set_id(self.uid);
        Promise::ok(())
    }

    fn name(
        &mut self,
        _params: hidio_capnp::h_i_d_i_o_server::NameParams,
        mut results: hidio_capnp::h_i_d_i_o_server::NameResults,
    ) -> Promise<(), Error> {
        results.get().set_name("hid-io-core");
        Promise::ok(())
    }

    fn log_files(
        &mut self,
        _params: hidio_capnp::h_i_d_i_o_server::LogFilesParams,
        mut results: hidio_capnp::h_i_d_i_o_server::LogFilesResults,
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
    mailbox: mailbox::Mailbox,
    node: Endpoint,
    auth: AuthLevel,
    subscriptions: Arc<RwLock<Subscriptions>>,
}

impl HIDIOImpl {
    fn new(
        mailbox: mailbox::Mailbox,
        node: Endpoint,
        auth: AuthLevel,
        subscriptions: Arc<RwLock<Subscriptions>>,
    ) -> HIDIOImpl {
        HIDIOImpl {
            mailbox,
            node,
            auth,
            subscriptions,
        }
    }
}

impl hidio_capnp::h_i_d_i_o::Server for HIDIOImpl {
    fn nodes(
        &mut self,
        _params: hidio_capnp::h_i_d_i_o::NodesParams,
        mut results: hidio_capnp::h_i_d_i_o::NodesResults,
    ) -> Promise<(), Error> {
        let nodes = self.mailbox.nodes.read().unwrap();
        let mut result = results.get().init_nodes((nodes.len()) as u32);
        for (i, n) in nodes.iter().enumerate() {
            let mut node = result.reborrow().get(i as u32);
            node.set_type(n.type_);
            node.set_name(&n.name);
            node.set_serial(&n.serial);
            node.set_id(n.uid);
            let mut node = node.init_node();
            match n.type_ {
                common_capnp::NodeType::HidioDaemon => {
                    node.set_daemon(capnp_rpc::new_client(DaemonNodeImpl::new(
                        self.mailbox.clone(),
                        self.node.clone(),
                        n.uid,
                        self.auth,
                        self.subscriptions.clone(),
                    )));
                }
                common_capnp::NodeType::UsbKeyboard | common_capnp::NodeType::BleKeyboard => {
                    node.set_keyboard(capnp_rpc::new_client(KeyboardNodeImpl::new(
                        self.mailbox.clone(),
                        self.node.clone(),
                        n.uid,
                        self.auth,
                        self.subscriptions.clone(),
                    )));
                }
                _ => {}
            }
        }
        Promise::ok(())
    }

    fn subscribe_nodes(
        &mut self,
        params: hidio_capnp::h_i_d_i_o::SubscribeNodesParams,
        mut results: hidio_capnp::h_i_d_i_o::SubscribeNodesResults,
    ) -> Promise<(), Error> {
        let id = self.subscriptions.read().unwrap().nodes_next_id;
        info!("Adding subscribeNodes watcher id #{}", id,);
        let client = pry!(pry!(params.get()).get_subscriber());
        self.subscriptions
            .write()
            .unwrap()
            .nodes
            .subscribers
            .insert(
                id,
                NodesSubscriberHandle {
                    client,
                    requests_in_flight: 0,
                    auth: self.auth,
                    node: self.node.clone(),
                },
            );

        results
            .get()
            .set_subscription(capnp_rpc::new_client(NodesSubscriptionImpl::new(
                self.mailbox.clone(),
                self.node.clone(),
                self.node.uid,
                self.subscriptions.clone(),
            )));

        self.subscriptions.write().unwrap().nodes_next_id += 1;
        Promise::ok(())
    }
}

struct NodesSubscriberHandle {
    client: hidio_capnp::h_i_d_i_o::nodes_subscriber::Client,
    requests_in_flight: i32,
    auth: AuthLevel,
    node: Endpoint,
}

struct NodesSubscriberMap {
    subscribers: HashMap<u64, NodesSubscriberHandle>,
}

impl NodesSubscriberMap {
    fn new() -> NodesSubscriberMap {
        NodesSubscriberMap {
            subscribers: HashMap::new(),
        }
    }
}

struct NodesSubscriptionImpl {
    _mailbox: mailbox::Mailbox,
    _node: Endpoint, // API Node information
    uid: u64,
    subscriptions: Arc<RwLock<Subscriptions>>,
}

impl NodesSubscriptionImpl {
    fn new(
        mailbox: mailbox::Mailbox,
        node: Endpoint,
        uid: u64,
        subscriptions: Arc<RwLock<Subscriptions>>,
    ) -> NodesSubscriptionImpl {
        NodesSubscriptionImpl {
            _mailbox: mailbox,
            _node: node,
            uid,
            subscriptions,
        }
    }
}

impl Drop for NodesSubscriptionImpl {
    fn drop(&mut self) {
        info!("Subscription dropped id: {}", self.uid);
        self.subscriptions
            .write()
            .unwrap()
            .nodes
            .subscribers
            .remove(&self.uid);
    }
}

impl hidio_capnp::h_i_d_i_o::nodes_subscription::Server for NodesSubscriptionImpl {}

struct KeyboardNodeImpl {
    mailbox: mailbox::Mailbox,
    node: Endpoint, // API Node information
    uid: u64,       // Device uid
    auth: AuthLevel,
    subscriptions: Arc<RwLock<Subscriptions>>,
}

impl KeyboardNodeImpl {
    fn new(
        mailbox: mailbox::Mailbox,
        node: Endpoint,
        uid: u64,
        auth: AuthLevel,
        subscriptions: Arc<RwLock<Subscriptions>>,
    ) -> KeyboardNodeImpl {
        KeyboardNodeImpl {
            mailbox,
            node,
            uid,
            auth,
            subscriptions,
        }
    }
}

impl common_capnp::node::Server for KeyboardNodeImpl {}

impl hidio_capnp::node::Server for KeyboardNodeImpl {
    fn cli_command(
        &mut self,
        params: hidio_capnp::node::CliCommandParams,
        _results: hidio_capnp::node::CliCommandResults,
    ) -> Promise<(), Error> {
        match self.auth {
            AuthLevel::Secure | AuthLevel::Debug => {
                let params = params.get().unwrap();
                let cmd = params.get_command().unwrap();
                let src = mailbox::Address::ApiCapnp { uid: self.node.uid };
                let dst = mailbox::Address::DeviceHidio { uid: self.uid };
                self.mailbox.send_command(
                    src,
                    dst,
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

    fn sleep_mode(
        &mut self,
        _params: hidio_capnp::node::SleepModeParams,
        mut results: hidio_capnp::node::SleepModeResults,
    ) -> Promise<(), Error> {
        match self.auth {
            AuthLevel::Secure | AuthLevel::Debug => {
                let src = mailbox::Address::ApiCapnp { uid: self.node.uid };
                let dst = mailbox::Address::DeviceHidio { uid: self.uid };
                self.mailbox
                    .send_command(src, dst, HIDIOCommandID::SleepMode, vec![]);

                // Wait for ACK/NAK
                let res = tokio::runtime::Runtime::new()
                    .unwrap()
                    .block_on(self.mailbox.ack_wait(dst, HIDIOCommandID::FlashMode, 0));
                match res {
                    Ok(_msg) => Promise::ok(()),
                    Err(mailbox::AckWaitError::TooManySyncs) => Promise::err(capnp::Error {
                        kind: capnp::ErrorKind::Failed,
                        description: "sleep_mode - Too many syncs...timeout".to_string(),
                    }),
                    Err(mailbox::AckWaitError::NAKReceived { msg }) => {
                        match msg.data.data.first() {
                            Some(0x0) => {
                                let status = results.get().init_status();
                                let mut error = status.init_error();
                                error.set_reason(hidio_capnp::node::sleep_mode_status::error::ErrorReason::NotSupported);
                                Promise::ok(())
                            }
                            Some(0x1) => {
                                let status = results.get().init_status();
                                let mut error = status.init_error();
                                error.set_reason(
                                    hidio_capnp::node::sleep_mode_status::error::ErrorReason::Disabled,
                                );
                                Promise::ok(())
                            }
                            Some(0x2) => {
                                let status = results.get().init_status();
                                let mut error = status.init_error();
                                error.set_reason(
                                    hidio_capnp::node::sleep_mode_status::error::ErrorReason::NotReady,
                                );
                                Promise::ok(())
                            }
                            Some(error_code) => Promise::err(capnp::Error {
                                kind: capnp::ErrorKind::Failed,
                                description: format!("sleep_mode - Unknown error {}", error_code),
                            }),
                            None => Promise::err(capnp::Error {
                                kind: capnp::ErrorKind::Failed,
                                description: "sleep_mode - Invalid NAK packet size".to_string(),
                            }),
                        }
                    }
                    Err(mailbox::AckWaitError::Invalid) => Promise::err(capnp::Error {
                        kind: capnp::ErrorKind::Failed,
                        description: "sleep_mode - Invalid response".to_string(),
                    }),
                }
            }
            _ => Promise::err(capnp::Error {
                kind: capnp::ErrorKind::Failed,
                description: "Insufficient authorization level".to_string(),
            }),
        }
    }

    fn flash_mode(
        &mut self,
        _params: hidio_capnp::node::FlashModeParams,
        mut results: hidio_capnp::node::FlashModeResults,
    ) -> Promise<(), Error> {
        match self.auth {
            AuthLevel::Secure | AuthLevel::Debug => {
                let src = mailbox::Address::ApiCapnp { uid: self.node.uid };
                let dst = mailbox::Address::DeviceHidio { uid: self.uid };
                // Send command
                self.mailbox
                    .send_command(src, dst, HIDIOCommandID::FlashMode, vec![]);

                // Wait for ACK/NAK
                let res = tokio::runtime::Runtime::new()
                    .unwrap()
                    .block_on(self.mailbox.ack_wait(dst, HIDIOCommandID::FlashMode, 0));
                match res {
                    Ok(msg) => {
                        // Convert byte stream to u16 TODO Should handle better
                        let scancode = ((msg.data.data[0] as u16) << 8) | msg.data.data[1] as u16;
                        let status = results.get().init_status();
                        let mut success = status.init_success();
                        success.set_scan_code(scancode);
                        Promise::ok(())
                    }
                    Err(mailbox::AckWaitError::TooManySyncs) => Promise::err(capnp::Error {
                        kind: capnp::ErrorKind::Failed,
                        description: "flash_mode - Too many syncs...timeout".to_string(),
                    }),
                    Err(mailbox::AckWaitError::NAKReceived { msg }) => {
                        match msg.data.data.first() {
                            Some(0x0) => {
                                let status = results.get().init_status();
                                let mut error = status.init_error();
                                error.set_reason(hidio_capnp::node::flash_mode_status::error::ErrorReason::NotSupported);
                                Promise::ok(())
                            }
                            Some(0x1) => {
                                let status = results.get().init_status();
                                let mut error = status.init_error();
                                error.set_reason(
                                    hidio_capnp::node::flash_mode_status::error::ErrorReason::Disabled,
                                );
                                Promise::ok(())
                            }
                            Some(error_code) => Promise::err(capnp::Error {
                                kind: capnp::ErrorKind::Failed,
                                description: format!("flash_mode - Unknown error {}", error_code),
                            }),
                            None => Promise::err(capnp::Error {
                                kind: capnp::ErrorKind::Failed,
                                description: "flash_mode - Invalid NAK packet size".to_string(),
                            }),
                        }
                    }
                    Err(mailbox::AckWaitError::Invalid) => Promise::err(capnp::Error {
                        kind: capnp::ErrorKind::Failed,
                        description: "flash_mode - Invalid response".to_string(),
                    }),
                }
            }
            _ => Promise::err(capnp::Error {
                kind: capnp::ErrorKind::Failed,
                description: "Insufficient authorization level".to_string(),
            }),
        }
    }
}

impl keyboard_capnp::keyboard::Server for KeyboardNodeImpl {
    fn subscribe(
        &mut self,
        params: keyboard_capnp::keyboard::SubscribeParams,
        mut results: keyboard_capnp::keyboard::SubscribeResults,
    ) -> Promise<(), Error> {
        let sid = self.subscriptions.read().unwrap().keyboard_node_next_id;
        info!("Adding KeyboardNode watcher id #{}", sid,);
        let client = pry!(pry!(params.get()).get_subscriber());
        self.subscriptions
            .write()
            .unwrap()
            .keyboard_node
            .subscribers
            .insert(
                sid,
                KeyboardSubscriberHandle {
                    client,
                    _requests_in_flight: 0,
                    _auth: self.auth,
                    _node: self.node.clone(),
                    uid: self.uid,
                },
            );

        results
            .get()
            .set_subscription(capnp_rpc::new_client(KeyboardSubscriptionImpl::new(
                self.mailbox.clone(),
                self.node.clone(),
                self.uid,
                sid,
                self.subscriptions.clone(),
            )));

        self.subscriptions.write().unwrap().keyboard_node_next_id += 1;
        Promise::ok(())
    }
}

struct KeyboardSubscriberHandle {
    client: keyboard_capnp::keyboard::subscriber::Client,
    _requests_in_flight: i32,
    _auth: AuthLevel,
    _node: Endpoint,
    uid: u64,
}

struct KeyboardSubscriberMap {
    subscribers: HashMap<u64, KeyboardSubscriberHandle>,
}

impl KeyboardSubscriberMap {
    fn new() -> KeyboardSubscriberMap {
        KeyboardSubscriberMap {
            subscribers: HashMap::new(),
        }
    }
}

struct KeyboardSubscriptionImpl {
    mailbox: mailbox::Mailbox,
    _node: Endpoint, // API Node information
    uid: u64,        // Device endpoint uid
    sid: u64,        // Subscription id
    subscriptions: Arc<RwLock<Subscriptions>>,
}

impl KeyboardSubscriptionImpl {
    fn new(
        mailbox: mailbox::Mailbox,
        node: Endpoint,
        uid: u64,
        sid: u64,
        subscriptions: Arc<RwLock<Subscriptions>>,
    ) -> KeyboardSubscriptionImpl {
        KeyboardSubscriptionImpl {
            mailbox,
            _node: node,
            uid,
            sid,
            subscriptions,
        }
    }
}

impl Drop for KeyboardSubscriptionImpl {
    fn drop(&mut self) {
        info!("KeyboardSubscription dropped id: {}", self.uid);
        self.mailbox.drop_subscriber(self.uid, self.sid);
        self.subscriptions
            .write()
            .unwrap()
            .keyboard_node
            .subscribers
            .remove(&self.uid);
    }
}

impl keyboard_capnp::keyboard::subscription::Server for KeyboardSubscriptionImpl {}

struct DaemonNodeImpl {
    mailbox: mailbox::Mailbox,
    node: Endpoint, // API Node information
    uid: u64,       // Device uid
    auth: AuthLevel,
    subscriptions: Arc<RwLock<Subscriptions>>,
}

impl DaemonNodeImpl {
    fn new(
        mailbox: mailbox::Mailbox,
        node: Endpoint,
        uid: u64,
        auth: AuthLevel,
        subscriptions: Arc<RwLock<Subscriptions>>,
    ) -> DaemonNodeImpl {
        DaemonNodeImpl {
            mailbox,
            node,
            uid,
            auth,
            subscriptions,
        }
    }
}

impl common_capnp::node::Server for DaemonNodeImpl {}

impl daemon_capnp::daemon::Server for DaemonNodeImpl {
    fn subscribe(
        &mut self,
        params: daemon_capnp::daemon::SubscribeParams,
        mut results: daemon_capnp::daemon::SubscribeResults,
    ) -> Promise<(), Error> {
        let id = self.subscriptions.read().unwrap().daemon_node_next_id;
        info!("Adding DaemonNode watcher id #{}", id,);
        let client = pry!(pry!(params.get()).get_subscriber());
        self.subscriptions
            .write()
            .unwrap()
            .daemon_node
            .subscribers
            .insert(
                id,
                DaemonSubscriberHandle {
                    _client: client,
                    _requests_in_flight: 0,
                    _auth: self.auth,
                    _node: self.node.clone(),
                },
            );

        results
            .get()
            .set_subscription(capnp_rpc::new_client(DaemonSubscriptionImpl::new(
                self.mailbox.clone(),
                self.node.clone(),
                self.uid,
                self.subscriptions.clone(),
            )));

        self.subscriptions.write().unwrap().daemon_node_next_id += 1;
        Promise::ok(())
    }
}

struct DaemonSubscriberHandle {
    _client: daemon_capnp::daemon::subscriber::Client,
    _requests_in_flight: i32,
    _auth: AuthLevel,
    _node: Endpoint,
}

struct DaemonSubscriberMap {
    subscribers: HashMap<u64, DaemonSubscriberHandle>,
}

impl DaemonSubscriberMap {
    fn new() -> DaemonSubscriberMap {
        DaemonSubscriberMap {
            subscribers: HashMap::new(),
        }
    }
}

struct DaemonSubscriptionImpl {
    _mailbox: mailbox::Mailbox,
    _node: Endpoint, // API Node information
    uid: u64,
    subscriptions: Arc<RwLock<Subscriptions>>,
}

impl DaemonSubscriptionImpl {
    fn new(
        mailbox: mailbox::Mailbox,
        node: Endpoint,
        uid: u64,
        subscriptions: Arc<RwLock<Subscriptions>>,
    ) -> DaemonSubscriptionImpl {
        DaemonSubscriptionImpl {
            _mailbox: mailbox,
            _node: node,
            uid,
            subscriptions,
        }
    }
}

impl Drop for DaemonSubscriptionImpl {
    fn drop(&mut self) {
        info!("Subscription dropped id: {}", self.uid);
        self.subscriptions
            .write()
            .unwrap()
            .daemon_node
            .subscribers
            .remove(&self.uid);
    }
}

impl daemon_capnp::daemon::subscription::Server for DaemonSubscriptionImpl {}

/// Capnproto Server
async fn server_bind(
    mailbox: mailbox::Mailbox,
    subscriptions: Arc<RwLock<Subscriptions>>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Open secured capnproto interface
    let addr = LISTEN_ADDR
        .to_socket_addrs()?
        .next()
        .expect("could not parse address");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    println!("API: Listening on {}", addr);

    // Generate new self-signed public/private key
    // Private key is not written to disk and generated each time
    let subject_alt_names = vec!["localhost".to_string()];
    let pair = generate_simple_self_signed(subject_alt_names).unwrap();

    let cert = Certificate(pair.serialize_der().unwrap());
    let pkey = PrivateKey(pair.serialize_private_key_der());
    let mut config = ServerConfig::new(NoClientAuth::new());
    config.set_single_cert(vec![cert], pkey).unwrap();
    let acceptor = TlsAcceptor::from(Arc::new(config));

    let nodes = mailbox.nodes.clone();
    let last_uid = mailbox.last_uid.clone();

    let connections: Arc<RwLock<HashMap<u64, Vec<u64>>>> = Arc::new(RwLock::new(HashMap::new()));

    loop {
        if !RUNNING.load(Ordering::SeqCst) {
            break Ok(());
        }

        // Setup connection abort
        // TODO - Test ongoing connections once they are working!
        let (abort_handle, abort_registration) = futures::future::AbortHandle::new_pair();
        tokio::spawn(async move {
            loop {
                if !RUNNING.load(Ordering::SeqCst) {
                    abort_handle.abort();
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        });
        // Setup TLS stream
        let stream_abortable =
            futures::future::Abortable::new(listener.accept(), abort_registration);
        let (stream, _addr) = stream_abortable.await??;
        stream.set_nodelay(true)?;
        let acceptor = acceptor.clone();
        let stream = acceptor.accept(stream).await?;

        // Save connection address for later
        let addr = stream.get_ref().0.peer_addr().ok().unwrap();

        // Setup reader/writer stream pair
        let (reader, writer) = futures_util::io::AsyncReadExt::split(
            tokio_util::compat::Tokio02AsyncReadCompatExt::compat(stream),
        );

        // Assign a uid to the connection
        let uid = {
            // Increment
            (*last_uid.write().unwrap()) += 1;
            let this_uid = *last_uid.read().unwrap();
            connections
                .clone()
                .write()
                .unwrap()
                .insert(this_uid, vec![]);
            this_uid
        };

        // Initialize auth tokens
        let hidio_server = HIDIOServerImpl::new(
            mailbox.clone(),
            connections.clone(),
            uid,
            subscriptions.clone(),
        );

        // Setup capnproto server
        let hidio_server: hidio_capnp::h_i_d_i_o_server::Client =
            capnp_rpc::new_client(hidio_server);
        let network = twoparty::VatNetwork::new(
            reader,
            writer,
            rpc_twoparty_capnp::Side::Server,
            Default::default(),
        );

        // Setup capnproto RPC
        let connections = connections.clone();
        let nodes = nodes.clone();
        let rpc_system = RpcSystem::new(Box::new(network), Some(hidio_server.client));
        let disconnector = rpc_system.get_disconnector();
        let rpc_task = tokio::task::spawn_local(async move {
            let uid = uid.clone();
            let addr = addr.clone();
            let _rpc_system = Box::pin(rpc_system.map_err(|e| info!("rpc_system: {}", e)).map(
                move |_| {
                    info!("Connection closed:7185 - {:?} - uid:{}", addr, uid);

                    // Client disconnected, delete node
                    let connected_nodes = connections.read().unwrap()[&uid].clone();
                    nodes
                        .write()
                        .unwrap()
                        .retain(|x| !connected_nodes.contains(&x.uid));
                },
            ))
            .await;
        });

        // This task is needed if hid-io-core wants to gracefully exit while capnp rpc_systems are
        // still active.
        tokio::task::spawn_local(async move {
            loop {
                if !RUNNING.load(Ordering::SeqCst) {
                    disconnector.await.unwrap();
                    rpc_task.abort();
                    // Check if we aborted or just exited normally (i.e. task already complete)
                    match rpc_task.await {
                        Ok(_) => {}
                        Err(e) => {
                            if e.is_cancelled() {
                                warn!("Connection aborted:7185 - {:?} - uid:{}", addr, uid);
                            }
                            if e.is_panic() {
                                error!("Connection panic:7185 - {:?} - uid:{}", addr, uid);
                            }
                        }
                    };
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        });
    }
}

/// Capnproto node subscriptions
async fn server_subscriptions(
    mailbox: mailbox::Mailbox,
    subscriptions: Arc<RwLock<Subscriptions>>,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Setting up api subscriptions...");

    let mut last_node_refresh = Instant::now();
    let mut last_node_count = 0;

    let mut last_keyboard_next_id = 0;

    loop {
        if !RUNNING.load(Ordering::SeqCst) {
            break;
        }

        // Check for new keyboard node subscriptions
        while subscriptions.read().unwrap().keyboard_node_next_id > last_keyboard_next_id {
            // TODO
            // Locate the subscription
            let subscriptions = subscriptions.clone();
            let mailbox = mailbox.clone();

            // Spawn an task
            tokio::task::spawn_local(async move {
                // Subscribe to the mailbox to monitor for incoming messages
                let receiver = mailbox.sender.subscribe();

                // Wait on the appropriate message filter
                // TODO Use the cli option to monitor cli
                // TODO Stream or poll/await or both?
                // Filter: device uid
                debug!(
                    "WATCH ID: {:?}",
                    mailbox::Address::DeviceHidio {
                        uid: subscriptions
                            .read()
                            .unwrap()
                            .keyboard_node
                            .subscribers
                            .get(&last_keyboard_next_id)
                            .unwrap()
                            .uid
                    }
                );
                tokio::pin! {
                    let stream = receiver
                        .into_stream()
                        .filter(Result::is_ok).map(Result::unwrap)
                        .take_while(|msg|
                            msg.src != mailbox::Address::DropSubscription &&
                            msg.dst != mailbox::Address::CancelSubscription {
                                uid: subscriptions.read().unwrap().keyboard_node.subscribers.get(&last_keyboard_next_id).unwrap().uid,
                                sid: last_keyboard_next_id
                            }
                        )
                        .filter(|msg|
                            msg.src == mailbox::Address::DeviceHidio {
                                uid: subscriptions.read().unwrap().keyboard_node.subscribers.get(&last_keyboard_next_id).unwrap().uid
                            }
                        );
                }
                // Filter: cli command
                let mut stream =
                    stream.filter(|msg| msg.data.id == HIDIOCommandID::Terminal as u32);
                // Filters: kll trigger
                //let stream = stream.filter(|msg| msg.data.id == HIDIOCommandID::KLLState);
                // Filters: layer
                // TODO
                // Filters: host macro
                //let stream = stream.filter(|msg| msg.data.id == HIDIOCommandID::HostMacro);

                while let Some(msg) = stream.next().await {
                    debug!("{:?}", msg);
                    // TODO
                    //stream.await

                    // Forward message to api callback
                    // TODO requests in flight
                    let mut request = subscriptions
                        .read()
                        .unwrap()
                        .keyboard_node
                        .subscribers
                        .get(&last_keyboard_next_id)
                        .unwrap()
                        .client
                        .update_request();
                    request.get().set_time(
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .expect("Time went backwards")
                            .as_millis() as u64,
                    );
                    request.send().promise.await.unwrap(); // TODO requests in flight
                                                           // End task if (a) Quitting (b) Subscription is dropped
                }

                debug!("NOOOOO");
            });

            // Increment to the next subscription
            last_keyboard_next_id += 1;
        }

        // Check for new daemon node subscriptions
        // TODO

        let subscriptions1 = subscriptions.clone();

        // Determine most recent device addition
        let nodes = mailbox.nodes.clone();
        let mut nodes_update = false;
        let mut cur_node_count = 0;

        nodes.read().unwrap().iter().for_each(|endpoint| {
            if let Some(_duration) = endpoint.created.checked_duration_since(last_node_refresh) {
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

        // Only send updates when node list has changed
        if nodes_update {
            let sub_count = subscriptions.read().unwrap().nodes.subscribers.len();
            info!(
                "Node list update detected, pushing list to subscribers -> {}",
                sub_count
            );

            let subs = &mut subscriptions.write().unwrap().nodes.subscribers;
            for (&idx, mut subscriber) in subs.iter_mut() {
                if subscriber.requests_in_flight < 5 {
                    subscriber.requests_in_flight += 1;
                    let mut request = subscriber.client.nodes_update_request();
                    {
                        let mut c_nodes = request.get().init_nodes(last_node_count as u32);
                        for (i, n) in nodes.read().unwrap().iter().enumerate() {
                            let mut node = c_nodes.reborrow().get(i as u32);
                            node.set_type(n.type_);
                            node.set_name(&n.name);
                            node.set_serial(&n.serial);
                            node.set_id(n.uid);
                            let mut node = node.init_node();
                            match n.type_ {
                                common_capnp::NodeType::HidioDaemon => {
                                    node.set_daemon(capnp_rpc::new_client(DaemonNodeImpl::new(
                                        mailbox.clone(),
                                        subscriber.node.clone(),
                                        n.uid,
                                        subscriber.auth,
                                        subscriptions.clone(),
                                    )));
                                }
                                common_capnp::NodeType::UsbKeyboard
                                | common_capnp::NodeType::BleKeyboard => {
                                    node.set_keyboard(capnp_rpc::new_client(
                                        KeyboardNodeImpl::new(
                                            mailbox.clone(),
                                            subscriber.node.clone(),
                                            n.uid,
                                            subscriber.auth,
                                            subscriptions.clone(),
                                        ),
                                    ));
                                }
                                _ => {}
                            }
                        }
                    }

                    let subscriptions2 = subscriptions1.clone();
                    tokio::task::spawn_local(
                        request
                            .send()
                            .promise
                            .map(move |r| {
                                match r {
                                    Ok(_) => {
                                        if let Some(ref mut s) = subscriptions2
                                            .write()
                                            .unwrap()
                                            .nodes
                                            .subscribers
                                            .get_mut(&idx)
                                        {
                                            s.requests_in_flight -= 1;
                                        }
                                    }
                                    Err(e) => {
                                        warn!("Got error: {:?}. Dropping subscriber.", e);
                                        subscriptions2
                                            .write()
                                            .unwrap()
                                            .nodes
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
        } else {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    }

    Ok(())
}

/// Cap'n'Proto API Initialization
/// Sets up a localhost socket to deal with localhost-only API usages
/// Some API usages may require external authentication to validate trustworthiness
pub async fn initialize(mailbox: mailbox::Mailbox) -> Result<(), Box<dyn std::error::Error>> {
    info!("Initializing api...");
    let subscriptions = Arc::new(RwLock::new(Subscriptions::new()));

    let local = tokio::task::LocalSet::new();

    // Start server
    local.spawn_local(server_bind(mailbox.clone(), subscriptions.clone()));

    // Start subscription thread
    local.spawn_local(server_subscriptions(mailbox, subscriptions));
    local.await;

    Ok(())
}
