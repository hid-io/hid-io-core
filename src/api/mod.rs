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
use crate::protocol::hidio::HidIoCommandID;
use crate::protocol::hidio::HidIoPacketType;
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
#[derive(Clone, Copy, Debug, PartialEq)]
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

    #[cfg(target_os = "linux")]
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

    #[cfg(target_os = "linux")]
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
    pub uid: u64,
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

    pub fn set_daemonnode_params(&mut self) {
        self.name = "HID-IO Core Daemon Node".to_string();
        self.serial = format!("pid:{}", std::process::id());
    }

    pub fn set_evdev_params(&mut self, info: EvdevInfo) {
        self.evdev = info;
        self.name = self.name();
        self.serial = self.serial();
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

    // HidIo Keyboard node subscriptions
    keyboard_node_next_id: u64,
    keyboard_node: KeyboardSubscriberMap,

    // HidIo Daemon node subscriptions
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

struct HidIoServerImpl {
    mailbox: mailbox::Mailbox,
    connections: Arc<RwLock<HashMap<u64, Vec<u64>>>>,
    uid: u64,

    basic_key: String,
    auth_key: String,

    basic_key_file: tempfile::NamedTempFile,
    auth_key_file: tempfile::NamedTempFile,

    subscriptions: Arc<RwLock<Subscriptions>>,
}

impl HidIoServerImpl {
    fn new(
        mailbox: mailbox::Mailbox,
        connections: Arc<RwLock<HashMap<u64, Vec<u64>>>>,
        uid: u64,
        subscriptions: Arc<RwLock<Subscriptions>>,
    ) -> HidIoServerImpl {
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
        let basic_key = nanoid::nanoid!();
        let auth_key = nanoid::nanoid!();

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
        HidIoServerImpl {
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
    ) -> hidio_capnp::hid_io::Client {
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
        capnp_rpc::new_client(HidIoImpl::new(
            self.mailbox.clone(),
            node,
            auth,
            self.subscriptions.clone(),
        ))
    }
}

impl hidio_capnp::hid_io_server::Server for HidIoServerImpl {
    fn basic(
        &mut self,
        params: hidio_capnp::hid_io_server::BasicParams,
        mut results: hidio_capnp::hid_io_server::BasicResults,
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
        params: hidio_capnp::hid_io_server::AuthParams,
        mut results: hidio_capnp::hid_io_server::AuthResults,
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
        _params: hidio_capnp::hid_io_server::VersionParams,
        mut results: hidio_capnp::hid_io_server::VersionResults,
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
        _params: hidio_capnp::hid_io_server::AliveParams,
        mut results: hidio_capnp::hid_io_server::AliveResults,
    ) -> Promise<(), Error> {
        results.get().set_alive(true);
        Promise::ok(())
    }

    fn key(
        &mut self,
        _params: hidio_capnp::hid_io_server::KeyParams,
        mut results: hidio_capnp::hid_io_server::KeyResults,
    ) -> Promise<(), Error> {
        // Get and set fields
        let mut key = results.get().init_key();
        key.set_basic_key_path(&self.basic_key_file.path().display().to_string());
        key.set_auth_key_path(&self.auth_key_file.path().display().to_string());
        Promise::ok(())
    }

    fn id(
        &mut self,
        _params: hidio_capnp::hid_io_server::IdParams,
        mut results: hidio_capnp::hid_io_server::IdResults,
    ) -> Promise<(), Error> {
        results.get().set_id(self.uid);
        Promise::ok(())
    }

    fn name(
        &mut self,
        _params: hidio_capnp::hid_io_server::NameParams,
        mut results: hidio_capnp::hid_io_server::NameResults,
    ) -> Promise<(), Error> {
        results.get().set_name("hid-io-core");
        Promise::ok(())
    }

    fn log_files(
        &mut self,
        _params: hidio_capnp::hid_io_server::LogFilesParams,
        mut results: hidio_capnp::hid_io_server::LogFilesResults,
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

struct HidIoImpl {
    mailbox: mailbox::Mailbox,
    node: Endpoint,
    auth: AuthLevel,
    subscriptions: Arc<RwLock<Subscriptions>>,
}

impl HidIoImpl {
    fn new(
        mailbox: mailbox::Mailbox,
        node: Endpoint,
        auth: AuthLevel,
        subscriptions: Arc<RwLock<Subscriptions>>,
    ) -> HidIoImpl {
        HidIoImpl {
            mailbox,
            node,
            auth,
            subscriptions,
        }
    }
}

impl hidio_capnp::hid_io::Server for HidIoImpl {
    fn nodes(
        &mut self,
        _params: hidio_capnp::hid_io::NodesParams,
        mut results: hidio_capnp::hid_io::NodesResults,
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
        params: hidio_capnp::hid_io::SubscribeNodesParams,
        mut results: hidio_capnp::hid_io::SubscribeNodesResults,
    ) -> Promise<(), Error> {
        let sid = match self.subscriptions.read() {
            Ok(sub) => sub.nodes_next_id,
            Err(e) => {
                return Promise::err(capnp::Error {
                    kind: capnp::ErrorKind::Failed,
                    description: format!("Failed to get sid lock: {}", e),
                });
            }
        };
        info!(
            "Adding subscribeNodes watcher sid:{} uid:{}",
            sid, self.node.uid
        );
        let client = pry!(pry!(params.get()).get_subscriber());
        self.subscriptions
            .write()
            .unwrap()
            .nodes
            .subscribers
            .insert(
                sid,
                NodesSubscriberHandle {
                    client,
                    requests_in_flight: 0,
                    auth: self.auth,
                    node: self.node.clone(),
                    uid: self.node.uid,
                },
            );

        results
            .get()
            .set_subscription(capnp_rpc::new_client(NodesSubscriptionImpl::new(
                self.mailbox.clone(),
                self.node.clone(),
                self.node.uid,
                self.subscriptions.clone(),
                sid,
            )));

        self.subscriptions.write().unwrap().nodes_next_id += 1;
        Promise::ok(())
    }
}

struct NodesSubscriberHandle {
    client: hidio_capnp::hid_io::nodes_subscriber::Client,
    requests_in_flight: i32,
    auth: AuthLevel,
    node: Endpoint,
    uid: u64,
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
    mailbox: mailbox::Mailbox,
    _node: Endpoint, // API Node information
    uid: u64,
    subscriptions: Arc<RwLock<Subscriptions>>,
    sid: u64,
}

impl NodesSubscriptionImpl {
    fn new(
        mailbox: mailbox::Mailbox,
        node: Endpoint,
        uid: u64,
        subscriptions: Arc<RwLock<Subscriptions>>,
        sid: u64,
    ) -> NodesSubscriptionImpl {
        NodesSubscriptionImpl {
            mailbox,
            _node: node,
            uid,
            subscriptions,
            sid,
        }
    }
}

impl Drop for NodesSubscriptionImpl {
    fn drop(&mut self) {
        info!("subscribeNodes dropped uid:{} sid:{}", self.uid, self.sid);
        self.mailbox.drop_subscriber(self.uid, self.sid);
        self.subscriptions
            .write()
            .unwrap()
            .nodes
            .subscribers
            .remove(&self.sid);
    }
}

impl hidio_capnp::hid_io::nodes_subscription::Server for NodesSubscriptionImpl {}

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

    fn request_info_u16(&mut self, id: u8) -> Option<u16> {
        let src = mailbox::Address::ApiCapnp { uid: self.node.uid };
        let dst = mailbox::Address::DeviceHidio { uid: self.uid };

        // Send command
        let res = self
            .mailbox
            .try_send_command(src, dst, HidIoCommandID::GetInfo, vec![id], true);

        // Wait for ACK/NAK
        match res {
            Ok(msg) => {
                if let Some(msg) = msg {
                    let mut data = [0u8; 2];
                    data.clone_from_slice(&msg.data.data[..=2]);
                    Some(u16::from_le_bytes(data))
                } else {
                    None
                }
            }
            Err(_) => None,
        }
    }

    fn request_info_string(&mut self, id: u8) -> Option<String> {
        let src = mailbox::Address::ApiCapnp { uid: self.node.uid };
        let dst = mailbox::Address::DeviceHidio { uid: self.uid };

        // Send command
        let res = self
            .mailbox
            .try_send_command(src, dst, HidIoCommandID::GetInfo, vec![id], true);

        // Wait for ACK/NAK
        match res {
            Ok(msg) => {
                if let Some(msg) = msg {
                    match std::str::from_utf8(&msg.data.data) {
                        Ok(val) => Some(val.to_string()),
                        Err(_) => None,
                    }
                } else {
                    None
                }
            }
            Err(_) => None,
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
                match self.mailbox.try_send_command(
                    src,
                    dst,
                    HidIoCommandID::Terminal,
                    cmd.as_bytes().to_vec(),
                    //true,
                    false, // TODO ACK Should work (firmware bug?)
                ) {
                    Ok(_msg) => {
                        // TODO (HaaTa): FIXME This should have an ACK
                        /*
                        if let Some(msg) = msg {
                            Promise::ok(())
                        } else {
                            Promise::err(capnp::Error {
                            kind: capnp::ErrorKind::Failed,
                            description: format!("No ACK received (cli_command)"),
                            })
                        }
                        */
                        Promise::ok(())
                    }
                    Err(e) => Promise::err(capnp::Error {
                        kind: capnp::ErrorKind::Failed,
                        description: format!("Error (cli_command): {:?}", e),
                    }),
                }
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

                // Wait for ACK/NAK
                let res = self.mailbox.try_send_command(
                    src,
                    dst,
                    HidIoCommandID::SleepMode,
                    vec![],
                    true,
                );

                match res {
                    Ok(_msg) => Promise::ok(()),
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
                    Err(e) => Promise::err(capnp::Error {
                        kind: capnp::ErrorKind::Failed,
                        description: format!("Error (sleep_mode): {:?}", e),
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
                let res = self.mailbox.try_send_command(
                    src,
                    dst,
                    HidIoCommandID::FlashMode,
                    vec![],
                    true,
                );

                // Wait for ACK/NAK
                match res {
                    Ok(msg) => {
                        if let Some(msg) = msg {
                            // Convert byte stream to u16 TODO Should handle better
                            let scancode =
                                ((msg.data.data[0] as u16) << 8) | msg.data.data[1] as u16;
                            let status = results.get().init_status();
                            let mut success = status.init_success();
                            success.set_scan_code(scancode);
                            Promise::ok(())
                        } else {
                            Promise::err(capnp::Error {
                                kind: capnp::ErrorKind::Failed,
                                description: "Error no ACK (flash_mode)".to_string(),
                            })
                        }
                    }
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
                    Err(e) => Promise::err(capnp::Error {
                        kind: capnp::ErrorKind::Failed,
                        description: format!("Error (flash_mode): {:?}", e),
                    }),
                }
            }
            _ => Promise::err(capnp::Error {
                kind: capnp::ErrorKind::Failed,
                description: "Insufficient authorization level".to_string(),
            }),
        }
    }

    fn manufacturing_test(
        &mut self,
        params: hidio_capnp::node::ManufacturingTestParams,
        mut _results: hidio_capnp::node::ManufacturingTestResults,
    ) -> Promise<(), Error> {
        match self.auth {
            AuthLevel::Secure | AuthLevel::Debug => {
                let src = mailbox::Address::ApiCapnp { uid: self.node.uid };
                let dst = mailbox::Address::DeviceHidio { uid: self.uid };

                let params = params.get().unwrap();
                let mut data = params.get_cmd().to_le_bytes().to_vec();
                data.append(&mut params.get_arg().to_le_bytes().to_vec());

                // Send command
                let res = self.mailbox.try_send_command(
                    src,
                    dst,
                    HidIoCommandID::ManufacturingTest,
                    data,
                    true,
                );

                // Wait for ACK/NAK
                match res {
                    Ok(msg) => {
                        if let Some(_msg) = msg {
                            Promise::ok(())
                        } else {
                            Promise::err(capnp::Error {
                                kind: capnp::ErrorKind::Failed,
                                description: "Error no ACK (manufacturing_test)".to_string(),
                            })
                        }
                    }
                    Err(e) => Promise::err(capnp::Error {
                        kind: capnp::ErrorKind::Failed,
                        description: format!("Error (manufacturing_test): {:?}", e),
                    }),
                }
            }
            _ => Promise::err(capnp::Error {
                kind: capnp::ErrorKind::Failed,
                description: "Insufficient authorization level".to_string(),
            }),
        }
    }

    fn info(
        &mut self,
        _params: hidio_capnp::node::InfoParams,
        mut results: hidio_capnp::node::InfoResults,
    ) -> Promise<(), Error> {
        let mut info = results.get().init_info();

        // Get version info
        if let Some(val) = self.request_info_u16(0x00) {
            info.set_hidio_major_version(val)
        }
        if let Some(val) = self.request_info_u16(0x01) {
            info.set_hidio_minor_version(val)
        }
        if let Some(val) = self.request_info_u16(0x02) {
            info.set_hidio_patch_version(val)
        }

        // Get device info
        if let Some(val) = self.request_info_string(0x03) {
            info.set_device_name(&val)
        }
        if let Some(val) = self.request_info_string(0x04) {
            info.set_device_serial(&val)
        }
        if let Some(val) = self.request_info_string(0x05) {
            info.set_device_version(&val)
        }
        if let Some(val) = self.request_info_string(0x06) {
            info.set_device_mcu(&val)
        }
        if let Some(val) = self.request_info_string(0x09) {
            info.set_device_vendor(&val)
        }

        // Get firmware info
        if let Some(val) = self.request_info_string(0x07) {
            info.set_firmware_name(&val)
        }
        if let Some(val) = self.request_info_string(0x08) {
            info.set_firmware_version(&val)
        }

        Promise::ok(())
    }
}

impl keyboard_capnp::keyboard::Server for KeyboardNodeImpl {
    fn subscribe(
        &mut self,
        params: keyboard_capnp::keyboard::SubscribeParams,
        mut results: keyboard_capnp::keyboard::SubscribeResults,
    ) -> Promise<(), Error> {
        // First check to make sure we're actually trying to subscribe to something
        let _options = match pry!(params.get()).get_options() {
            Ok(options) => {
                if options.len() == 0 {
                    return Promise::err(capnp::Error {
                        kind: capnp::ErrorKind::Failed,
                        description: "No subscription options specified".to_string(),
                    });
                }
                // TODO Store/Setup options for KeyboardSubscriberHandle
                options
            }
            Err(e) => {
                return Promise::err(capnp::Error {
                    kind: capnp::ErrorKind::Failed,
                    description: format!("Error reading subscription options: {}", e),
                });
            }
        };

        let sid = self.subscriptions.read().unwrap().keyboard_node_next_id;
        info!("Adding KeyboardNode watcher sid:{} uid:{}", sid, self.uid);
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
        info!(
            "KeyboardNode watcher dropped uid:{} sid:{}",
            self.uid, self.sid
        );
        self.mailbox.drop_subscriber(self.uid, self.sid);
        self.subscriptions
            .write()
            .unwrap()
            .keyboard_node
            .subscribers
            .remove(&self.sid);
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
        let sid = self.subscriptions.read().unwrap().daemon_node_next_id;
        info!("Adding DaemonNode watcher sid:{} uid:{}", sid, self.uid);
        let client = pry!(pry!(params.get()).get_subscriber());
        self.subscriptions
            .write()
            .unwrap()
            .daemon_node
            .subscribers
            .insert(
                sid,
                DaemonSubscriberHandle {
                    client,
                    _auth: self.auth,
                    _node: self.node.clone(),
                    uid: self.uid,
                },
            );

        results
            .get()
            .set_subscription(capnp_rpc::new_client(DaemonSubscriptionImpl::new(
                self.mailbox.clone(),
                self.node.clone(),
                self.uid,
                self.subscriptions.clone(),
                sid,
            )));

        self.subscriptions.write().unwrap().daemon_node_next_id += 1;
        Promise::ok(())
    }

    fn unicode_string(
        &mut self,
        params: daemon_capnp::daemon::UnicodeStringParams,
        mut _results: daemon_capnp::daemon::UnicodeStringResults,
    ) -> Promise<(), Error> {
        let params = params.get().unwrap();
        let string = params.get_string().unwrap();
        let src = mailbox::Address::ApiCapnp { uid: self.node.uid };
        let dst = mailbox::Address::Module;

        match self.mailbox.try_send_command(
            src,
            dst,
            HidIoCommandID::UnicodeText,
            string.as_bytes().to_vec(),
            true,
        ) {
            Ok(msg) => {
                if let Some(_msg) = msg {
                    Promise::ok(())
                } else {
                    Promise::err(capnp::Error {
                        kind: capnp::ErrorKind::Failed,
                        description: "No ACK received (unicode_string)".to_string(),
                    })
                }
            }
            Err(e) => Promise::err(capnp::Error {
                kind: capnp::ErrorKind::Failed,
                description: format!("Error (unicode_string): {:?}", e),
            }),
        }
    }

    fn unicode_keys(
        &mut self,
        params: daemon_capnp::daemon::UnicodeKeysParams,
        mut _results: daemon_capnp::daemon::UnicodeKeysResults,
    ) -> Promise<(), Error> {
        let params = params.get().unwrap();
        let string = params.get_characters().unwrap();
        let src = mailbox::Address::ApiCapnp { uid: self.node.uid };
        let dst = mailbox::Address::Module;

        match self.mailbox.try_send_command(
            src,
            dst,
            HidIoCommandID::UnicodeKey,
            string.as_bytes().to_vec(),
            true,
        ) {
            Ok(msg) => {
                if let Some(_msg) = msg {
                    Promise::ok(())
                } else {
                    Promise::err(capnp::Error {
                        kind: capnp::ErrorKind::Failed,
                        description: "No ACK received (unicode_keys)".to_string(),
                    })
                }
            }
            Err(e) => Promise::err(capnp::Error {
                kind: capnp::ErrorKind::Failed,
                description: format!("Error (unicode_keys): {:?}", e),
            }),
        }
    }

    fn info(
        &mut self,
        _params: daemon_capnp::daemon::InfoParams,
        mut results: daemon_capnp::daemon::InfoResults,
    ) -> Promise<(), Error> {
        let mut info = results.get().init_info();
        // Set version info
        info.set_hidio_major_version(built_info::PKG_VERSION_MAJOR.parse::<u16>().unwrap());
        info.set_hidio_minor_version(built_info::PKG_VERSION_MINOR.parse::<u16>().unwrap());
        info.set_hidio_patch_version(built_info::PKG_VERSION_PATCH.parse::<u16>().unwrap());

        // Set OS info
        info.set_os(built_info::CFG_OS);
        info.set_os_version(&sys_info::os_release().unwrap());

        // Set daemon name
        info.set_host_name(built_info::PKG_NAME);
        Promise::ok(())
    }
}

struct DaemonSubscriberHandle {
    client: daemon_capnp::daemon::subscriber::Client,
    _auth: AuthLevel,
    _node: Endpoint,
    uid: u64,
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
    mailbox: mailbox::Mailbox,
    _node: Endpoint, // API Node information
    uid: u64,
    subscriptions: Arc<RwLock<Subscriptions>>,
    sid: u64,
}

impl DaemonSubscriptionImpl {
    fn new(
        mailbox: mailbox::Mailbox,
        node: Endpoint,
        uid: u64,
        subscriptions: Arc<RwLock<Subscriptions>>,
        sid: u64,
    ) -> DaemonSubscriptionImpl {
        DaemonSubscriptionImpl {
            mailbox,
            _node: node,
            uid,
            subscriptions,
            sid,
        }
    }
}

impl Drop for DaemonSubscriptionImpl {
    fn drop(&mut self) {
        info!(
            "DaemonNode subscription dropped sid:{} uid:{}",
            self.sid, self.uid
        );
        self.mailbox.drop_subscriber(self.uid, self.sid);
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
        let hidio_server = HidIoServerImpl::new(
            mailbox.clone(),
            connections.clone(),
            uid,
            subscriptions.clone(),
        );

        // Setup capnproto server
        let hidio_server: hidio_capnp::hid_io_server::Client = capnp_rpc::new_client(hidio_server);
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

/// Daemon node subscriptions
async fn server_subscriptions_daemon(
    mailbox: mailbox::Mailbox,
    subscriptions: Arc<RwLock<Subscriptions>>,
    mut last_daemon_next_id: u64,
) -> Result<u64, Box<dyn std::error::Error>> {
    while subscriptions.read().unwrap().daemon_node_next_id > last_daemon_next_id {
        // Locate the subscription
        let subscriptions = subscriptions.clone();
        let mailbox = mailbox.clone();

        // Spawn an task
        tokio::task::spawn_local(async move {
            // Subscribe to the mailbox to monitor for incoming messages
            let receiver = mailbox.sender.subscribe();

            debug!(
                "daemonwatcher active uid:{:?}",
                mailbox::Address::DeviceHidio {
                    uid: subscriptions
                        .read()
                        .unwrap()
                        .daemon_node
                        .subscribers
                        .get(&last_daemon_next_id)
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
                            uid: subscriptions.read().unwrap().daemon_node.subscribers.get(&last_daemon_next_id).unwrap().uid,
                            sid: last_daemon_next_id
                        }
                    )
                    .take_while(|msg|
                        msg.src != mailbox::Address::DropSubscription &&
                        msg.dst != mailbox::Address::CancelAllSubscriptions
                    )
                    .filter(|msg|
                        msg.src == mailbox::Address::DeviceHidio {
                            uid: subscriptions.read().unwrap().daemon_node.subscribers.get(&last_daemon_next_id).unwrap().uid
                        }
                    );
            }

            // Filter: TODO

            // TODO Split into multiple stream paths? Or just handle here?
            while let Some(msg) = stream.next().await {
                debug!("DISDAM {:?}", msg);

                // Forward message to api callback
                let mut request = subscriptions
                    .read()
                    .unwrap()
                    .daemon_node
                    .subscribers
                    .get(&last_daemon_next_id)
                    .unwrap()
                    .client
                    .update_request();

                // Build Signal message
                let mut signal = request.get().init_signal();
                signal.set_time(
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .expect("Time went backwards")
                        .as_millis() as u64,
                );

                // Block on each send, drop subscription on failure
                if let Err(e) = request.send().promise.await {
                    warn!("daemonwatcher packet error: {:?}. Dropping subscriber.", e);
                    subscriptions
                        .write()
                        .unwrap()
                        .nodes
                        .subscribers
                        .remove(&last_daemon_next_id);
                    break;
                }
            }
        });

        // Increment to the next subscription
        last_daemon_next_id += 1;
    }

    Ok(last_daemon_next_id)
}

/// Keyboard node subscriptions
async fn server_subscriptions_keyboard(
    mailbox: mailbox::Mailbox,
    subscriptions: Arc<RwLock<Subscriptions>>,
    mut last_keyboard_next_id: u64,
) -> Result<u64, Box<dyn std::error::Error>> {
    while subscriptions.read().unwrap().keyboard_node_next_id > last_keyboard_next_id {
        // Locate the subscription
        let subscriptions = subscriptions.clone();
        let mailbox = mailbox.clone();

        // Spawn an task
        tokio::task::spawn_local(async move {
            // Subscribe to the mailbox to monitor for incoming messages
            let receiver = mailbox.sender.subscribe();

            debug!(
                "keyboardwatcher active uid:{:?}",
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
                    .take_while(|msg|
                        msg.src != mailbox::Address::DropSubscription &&
                        msg.dst != mailbox::Address::CancelAllSubscriptions
                    )
                    .filter(|msg|
                        msg.src == mailbox::Address::DeviceHidio {
                            uid: subscriptions.read().unwrap().keyboard_node.subscribers.get(&last_keyboard_next_id).unwrap().uid
                        }
                    );
            }
            // Filter: cli command
            let mut stream = stream.filter(|msg| msg.data.id == HidIoCommandID::Terminal);
            // Filters: kll trigger
            //let stream = stream.filter(|msg| msg.data.id == HidIoCommandID::KLLState);
            // Filters: layer
            // TODO
            // Filters: host macro
            //let stream = stream.filter(|msg| msg.data.id == HidIoCommandID::HostMacro);

            // TODO Split into multiple stream paths? Or just handle here?
            while let Some(msg) = stream.next().await {
                // Forward message to api callback
                let mut request = subscriptions
                    .read()
                    .unwrap()
                    .keyboard_node
                    .subscribers
                    .get(&last_keyboard_next_id)
                    .unwrap()
                    .client
                    .update_request();

                // Build Signal message
                let mut signal = request.get().init_signal();
                signal.set_time(
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .expect("Time went backwards")
                        .as_millis() as u64,
                );
                signal
                    .init_data()
                    .init_cli()
                    .set_output(&String::from_utf8_lossy(&msg.data.data));

                // Block on each send, drop subscription on failure
                if let Err(e) = request.send().promise.await {
                    warn!(
                        "keyboardwatcher packet error: {:?}. Dropping subscriber.",
                        e
                    );
                    subscriptions
                        .write()
                        .unwrap()
                        .nodes
                        .subscribers
                        .remove(&last_keyboard_next_id);
                    break;
                }
            }
        });

        // Increment to the next subscription
        last_keyboard_next_id += 1;
    }

    Ok(last_keyboard_next_id)
}

/// hidiowatcher subscriptions
async fn server_subscriptions_hidiowatcher(
    mailbox: mailbox::Mailbox,
    subscriptions: Arc<RwLock<Subscriptions>>,
    mut last_node_next_id: u64,
) -> Result<u64, Box<dyn std::error::Error>> {
    while subscriptions.read().unwrap().nodes_next_id > last_node_next_id {
        // Make sure we have Debug authlevel before creating watcher
        if subscriptions
            .clone()
            .read()
            .unwrap()
            .nodes
            .subscribers
            .get(&last_node_next_id)
            .unwrap()
            .auth
            != AuthLevel::Debug
        {
            // Skip to the next node id
            last_node_next_id += 1;
            continue;
        }

        // Locate the subscription
        let subscriptions = subscriptions.clone();
        let mailbox = mailbox.clone();

        // Spawn an task
        tokio::task::spawn_local(async move {
            // Subscribe to the mailbox to monitor for incoming messages
            let receiver = mailbox.sender.subscribe();

            debug!(
                "hidiowatcher active uid:{:?}",
                mailbox::Address::DeviceHidio {
                    uid: subscriptions
                        .read()
                        .unwrap()
                        .nodes
                        .subscribers
                        .get(&last_node_next_id)
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
                            uid: subscriptions.read().unwrap().nodes.subscribers.get(&last_node_next_id).unwrap().uid,
                            sid: last_node_next_id
                        }
                    )
                    .take_while(|msg|
                        msg.src != mailbox::Address::DropSubscription &&
                        msg.dst != mailbox::Address::CancelAllSubscriptions
                    );
            }

            while let Some(msg) = stream.next().await {
                // Forward message to api callback
                let mut request = subscriptions
                    .read()
                    .unwrap()
                    .nodes
                    .subscribers
                    .get(&last_node_next_id)
                    .unwrap()
                    .client
                    .hidio_watcher_request();
                let mut packet = request.get().init_packet();
                packet.set_src(match msg.src {
                    mailbox::Address::ApiCapnp { uid } => uid,
                    mailbox::Address::CancelSubscription { uid, sid: _ } => uid,
                    mailbox::Address::DeviceHidio { uid } => uid,
                    mailbox::Address::DeviceHid { uid } => uid,
                    _ => 0,
                });
                packet.set_dst(match msg.dst {
                    mailbox::Address::ApiCapnp { uid } => uid,
                    mailbox::Address::CancelSubscription { uid, sid: _ } => uid,
                    mailbox::Address::DeviceHidio { uid } => uid,
                    mailbox::Address::DeviceHid { uid } => uid,
                    _ => 0,
                });
                packet.set_type(match msg.data.ptype {
                    HidIoPacketType::Data => hidio_capnp::hid_io::packet::Type::Data,
                    HidIoPacketType::NAData => hidio_capnp::hid_io::packet::Type::NaData,
                    HidIoPacketType::ACK => hidio_capnp::hid_io::packet::Type::Ack,
                    HidIoPacketType::NAK => hidio_capnp::hid_io::packet::Type::Nak,
                    _ => hidio_capnp::hid_io::packet::Type::Unknown,
                });
                packet.set_id(msg.data.id as u32);
                let mut data = packet.init_data(msg.data.data.len() as u32);
                for (index, elem) in msg.data.data.iter().enumerate() {
                    data.set(index as u32, *elem);
                }

                // Block on each send, drop subscription on failure
                if let Err(e) = request.send().promise.await {
                    warn!("hidiowatcher packet error: {:?}. Dropping subscriber.", e);
                    subscriptions
                        .write()
                        .unwrap()
                        .nodes
                        .subscribers
                        .remove(&last_node_next_id);
                    break;
                }
            }
        });

        // Increment to the next subscription
        last_node_next_id += 1;
    }

    Ok(last_node_next_id)
}

/// Capnproto node subscriptions
async fn server_subscriptions(
    mailbox: mailbox::Mailbox,
    subscriptions: Arc<RwLock<Subscriptions>>,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Setting up api subscriptions...");

    // Id references (keeps track of state)
    let mut last_node_refresh = Instant::now();
    let mut last_node_count = 0;

    let mut last_daemon_next_id = 0;
    let mut last_keyboard_next_id = 0;
    let mut last_node_next_id = 0;

    loop {
        if !RUNNING.load(Ordering::SeqCst) {
            // Send signal to all tokio subscription threads to exit
            mailbox.drop_all_subscribers();
            break;
        }

        // Check for new keyboard node subscriptions
        last_keyboard_next_id = server_subscriptions_keyboard(
            mailbox.clone(),
            subscriptions.clone(),
            last_keyboard_next_id,
        )
        .await
        .unwrap();

        // Check for new daemon node subscriptions
        last_daemon_next_id = server_subscriptions_daemon(
            mailbox.clone(),
            subscriptions.clone(),
            last_daemon_next_id,
        )
        .await
        .unwrap();

        // Check for new node subscriptions (hidio watcher)
        last_node_next_id = server_subscriptions_hidiowatcher(
            mailbox.clone(),
            subscriptions.clone(),
            last_node_next_id,
        )
        .await
        .unwrap();

        // Handle nodes list subscriptions
        // Uses a more traditional requests_in_flight model which limits the broadcasts per
        // subscriber if the connection is slow.
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

/// Supported Ids by this module
pub fn supported_ids() -> Vec<HidIoCommandID> {
    vec![HidIoCommandID::Terminal]
}

/// Cap'n'Proto API Initialization
/// Sets up a localhost socket to deal with localhost-only API usages
/// Some API usages may require external authentication to validate trustworthiness
pub async fn initialize(rt: Arc<tokio::runtime::Runtime>, mailbox: mailbox::Mailbox) {
    info!("Initializing api...");

    // This confusing block spawns a dedicated thread, and then runs a task LocalSet inside of it
    // This is required to avoid the use of the Send trait.
    // hid-io-core requires multiple threads like this which can dead-lock each other if run from
    // the same thread (which is the default behaviour of task LocalSet spawn_local)
    rt.clone()
        .spawn_blocking(move || {
            rt.block_on(async {
                let subscriptions = Arc::new(RwLock::new(Subscriptions::new()));

                let local = tokio::task::LocalSet::new();

                // Start server
                local.spawn_local(server_bind(mailbox.clone(), subscriptions.clone()));

                // Start subscription thread
                local.spawn_local(server_subscriptions(mailbox, subscriptions));
                local.await;
            });
        })
        .await
        .unwrap();
}
