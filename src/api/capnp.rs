#![cfg(feature = "api")]
/* Copyright (C) 2017-2022 by Jacob Alexander
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

use crate::api::*;
use crate::built_info;
use crate::mailbox;
use crate::RUNNING;
use ::capnp::capability::Promise;
use ::capnp::Error;
use capnp_rpc::{pry, rpc_twoparty_capnp, twoparty, RpcSystem};
use futures::{FutureExt, TryFutureExt};
use glob::glob;
use hid_io_protocol::commands::*;
use hid_io_protocol::{HidIoCommandId, HidIoPacketType};
use rcgen::generate_simple_self_signed;
use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::Write;
use std::net::ToSocketAddrs;
use std::sync::atomic::Ordering;
use std::sync::{Arc, RwLock};
use std::time::Instant;
use tokio_rustls::{
    rustls::{Certificate, PrivateKey, ServerConfig},
    TlsAcceptor,
};
use tokio_stream::{wrappers::BroadcastStream, StreamExt};

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
            common_capnp::NodeType::Unknown => write!(f, "Unknown"),
        }
    }
}

impl std::fmt::Display for hidio_capnp::hid_io::packet::Type {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            hidio_capnp::hid_io::packet::Type::Data => write!(f, "Data"),
            hidio_capnp::hid_io::packet::Type::Ack => write!(f, "Ack"),
            hidio_capnp::hid_io::packet::Type::Nak => write!(f, "Nak"),
            hidio_capnp::hid_io::packet::Type::NaData => write!(f, "NaData"),
            hidio_capnp::hid_io::packet::Type::Unknown => write!(f, "Unknown"),
        }
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

    basic_key_dir: tempfile::TempDir,
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
        let basic_key_dir = tempfile::Builder::new()
            .prefix("hidio")
            .tempdir()
            .expect("Unable to create dir");
        let mut basic_key_file =
            File::create(basic_key_dir.path().join("key")).expect("Unable to create file");

        // Create temp file for auth key
        // Only this user can read the auth key
        let mut auth_key_file = tempfile::Builder::new()
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

            basic_key_dir,
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
                kind: ::capnp::ErrorKind::Failed,
                description: "Authentication denied (basic)".to_string(),
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
                kind: ::capnp::ErrorKind::Failed,
                description: "Authentication denied (auth)".to_string(),
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
        version.set_buildtime(built_info::BUILT_TIME_UTC);
        version.set_serverarch(built_info::TARGET);
        version.set_compilerversion(built_info::RUSTC_VERSION);
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
        key.set_basic_key_path(&self.basic_key_dir.path().join("key").display().to_string());
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
        #[allow(clippy::significant_drop_in_scrutinee)]
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
                    kind: ::capnp::ErrorKind::Failed,
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
                let command = heapless::String::from(params.get_command().unwrap());
                let src = mailbox::Address::ApiCapnp { uid: self.node.uid };
                let dst = mailbox::Address::DeviceHidio { uid: self.uid };

                struct CommandInterface {
                    src: mailbox::Address,
                    dst: mailbox::Address,
                    mailbox: mailbox::Mailbox,
                    result: Result<h0031::Ack, h0031::Nak>,
                }
                impl
                    Commands<
                        { mailbox::HIDIO_PKT_BUF_DATA_SIZE },
                        { mailbox::HIDIO_PKT_BUF_DATA_SIZE - 1 },
                        { mailbox::HIDIO_PKT_BUF_DATA_SIZE - 2 },
                        { mailbox::HIDIO_PKT_BUF_DATA_SIZE - 4 },
                        1, // TODO(HaaTa): https://github.com/japaric/heapless/issues/252
                    > for CommandInterface
                {
                    fn tx_packetbuffer_send(
                        &mut self,
                        buf: &mut mailbox::HidIoPacketBuffer,
                    ) -> Result<(), CommandError> {
                        if let Some(rcvmsg) = self.mailbox.try_send_message(mailbox::Message {
                            src: self.src,
                            dst: self.dst,
                            data: buf.clone(),
                        })? {
                            // Handle ack/nak
                            self.rx_message_handling(rcvmsg.data)?;
                        }
                        Ok(())
                    }
                    fn h0031_terminalcmd_ack(
                        &mut self,
                        data: h0031::Ack,
                    ) -> Result<(), CommandError> {
                        self.result = Ok(data);
                        Ok(())
                    }
                    fn h0031_terminalcmd_nak(
                        &mut self,
                        data: h0031::Nak,
                    ) -> Result<(), CommandError> {
                        self.result = Err(data);
                        Ok(())
                    }
                }
                let mut intf = CommandInterface {
                    src,
                    dst,
                    mailbox: self.mailbox.clone(),
                    result: Err(h0031::Nak {}),
                };

                // Send command
                let cmd = h0031::Cmd { command };
                if let Err(e) = intf.h0031_terminalcmd(cmd.clone(), false) {
                    return Promise::err(capnp::Error {
                        kind: ::capnp::ErrorKind::Failed,
                        description: format!("Error (cli_command): {:?} {:?}", cmd, e),
                    });
                }

                // Wait for Ack/Nak
                match intf.result {
                    Ok(_msg) => Promise::ok(()),
                    Err(e) => Promise::err(capnp::Error {
                        kind: ::capnp::ErrorKind::Failed,
                        description: format!("Error (cli_command): {:?}", e),
                    }),
                }
            }
            _ => Promise::err(capnp::Error {
                kind: ::capnp::ErrorKind::Failed,
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

                struct CommandInterface {
                    src: mailbox::Address,
                    dst: mailbox::Address,
                    mailbox: mailbox::Mailbox,
                    result: Result<h001a::Ack, h001a::Nak>,
                }
                impl
                    Commands<
                        { mailbox::HIDIO_PKT_BUF_DATA_SIZE },
                        { mailbox::HIDIO_PKT_BUF_DATA_SIZE - 1 },
                        { mailbox::HIDIO_PKT_BUF_DATA_SIZE - 2 },
                        { mailbox::HIDIO_PKT_BUF_DATA_SIZE - 4 },
                        1, // TODO(HaaTa): https://github.com/japaric/heapless/issues/252
                    > for CommandInterface
                {
                    fn tx_packetbuffer_send(
                        &mut self,
                        buf: &mut mailbox::HidIoPacketBuffer,
                    ) -> Result<(), CommandError> {
                        if let Some(rcvmsg) = self.mailbox.try_send_message(mailbox::Message {
                            src: self.src,
                            dst: self.dst,
                            data: buf.clone(),
                        })? {
                            // Handle ack/nak
                            self.rx_message_handling(rcvmsg.data)?;
                        }
                        Ok(())
                    }
                    fn h001a_sleepmode_ack(
                        &mut self,
                        data: h001a::Ack,
                    ) -> Result<(), CommandError> {
                        self.result = Ok(data);
                        Ok(())
                    }
                    fn h001a_sleepmode_nak(
                        &mut self,
                        data: h001a::Nak,
                    ) -> Result<(), CommandError> {
                        self.result = Err(data);
                        Ok(())
                    }
                }
                let mut intf = CommandInterface {
                    src,
                    dst,
                    mailbox: self.mailbox.clone(),
                    result: Err(h001a::Nak {
                        error: h001a::Error::NotSupported,
                    }),
                };

                // Send command
                if let Err(e) = intf.h001a_sleepmode(h001a::Cmd {}) {
                    return Promise::err(capnp::Error {
                        kind: ::capnp::ErrorKind::Failed,
                        description: format!("Error (sleepmode): {:?}", e),
                    });
                }

                // Wait for Ack/Nak
                let status = results.get().init_status();
                match intf.result {
                    Ok(_msg) => Promise::ok(()),
                    Err(msg) => match msg.error {
                        h001a::Error::NotSupported => {
                            let mut error = status.init_error();
                            error.set_reason(hidio_capnp::node::sleep_mode_status::error::ErrorReason::NotSupported);
                            Promise::ok(())
                        }
                        h001a::Error::Disabled => {
                            let mut error = status.init_error();
                            error.set_reason(
                                hidio_capnp::node::sleep_mode_status::error::ErrorReason::Disabled,
                            );
                            Promise::ok(())
                        }
                        h001a::Error::NotReady => {
                            let mut error = status.init_error();
                            error.set_reason(
                                hidio_capnp::node::sleep_mode_status::error::ErrorReason::NotReady,
                            );
                            Promise::ok(())
                        }
                    },
                }
            }
            _ => Promise::err(capnp::Error {
                kind: ::capnp::ErrorKind::Failed,
                description: "Insufficient authorization level".to_string(),
            }),
        }
    }

    fn flash_mode(
        &mut self,
        _params: hidio_capnp::node::FlashModeParams,
        results: hidio_capnp::node::FlashModeResults,
    ) -> Promise<(), Error> {
        match self.auth {
            AuthLevel::Secure | AuthLevel::Debug => {
                let src = mailbox::Address::ApiCapnp { uid: self.node.uid };
                let dst = mailbox::Address::DeviceHidio { uid: self.uid };

                struct CommandInterface {
                    src: mailbox::Address,
                    dst: mailbox::Address,
                    mailbox: mailbox::Mailbox,
                    results: hidio_capnp::node::FlashModeResults,
                }
                impl
                    Commands<
                        { mailbox::HIDIO_PKT_BUF_DATA_SIZE },
                        { mailbox::HIDIO_PKT_BUF_DATA_SIZE - 1 },
                        { mailbox::HIDIO_PKT_BUF_DATA_SIZE - 2 },
                        { mailbox::HIDIO_PKT_BUF_DATA_SIZE - 4 },
                        1, // TODO(HaaTa): https://github.com/japaric/heapless/issues/252
                    > for CommandInterface
                {
                    fn tx_packetbuffer_send(
                        &mut self,
                        buf: &mut mailbox::HidIoPacketBuffer,
                    ) -> Result<(), CommandError> {
                        if let Some(rcvmsg) = self.mailbox.try_send_message(mailbox::Message {
                            src: self.src,
                            dst: self.dst,
                            data: buf.clone(),
                        })? {
                            // Handle ack/nak
                            self.rx_message_handling(rcvmsg.data)?;
                        }
                        Ok(())
                    }
                    fn h0016_flashmode_ack(
                        &mut self,
                        data: h0016::Ack,
                    ) -> Result<(), CommandError> {
                        let status = self.results.get().init_status();
                        let mut success = status.init_success();
                        success.set_scan_code(data.scancode);
                        Ok(())
                    }
                    fn h0016_flashmode_nak(
                        &mut self,
                        data: h0016::Nak,
                    ) -> Result<(), CommandError> {
                        let status = self.results.get().init_status();
                        match data.error {
                            h0016::Error::NotSupported => {
                                let mut error = status.init_error();
                                error.set_reason(hidio_capnp::node::flash_mode_status::error::ErrorReason::NotSupported);
                            }
                            h0016::Error::Disabled => {
                                let mut error = status.init_error();
                                error.set_reason(
                                    hidio_capnp::node::flash_mode_status::error::ErrorReason::Disabled,
                                );
                            }
                        }
                        Ok(())
                    }
                }
                let mut intf = CommandInterface {
                    src,
                    dst,
                    mailbox: self.mailbox.clone(),
                    results,
                };

                // Send command
                if let Err(e) = intf.h0016_flashmode(h0016::Cmd {}) {
                    return Promise::err(capnp::Error {
                        kind: ::capnp::ErrorKind::Failed,
                        description: format!("Error (flashmode): {:?}", e),
                    });
                }
                Promise::ok(())
            }
            _ => Promise::err(capnp::Error {
                kind: ::capnp::ErrorKind::Failed,
                description: "Insufficient authorization level".to_string(),
            }),
        }
    }

    fn manufacturing_test(
        &mut self,
        params: hidio_capnp::node::ManufacturingTestParams,
        results: hidio_capnp::node::ManufacturingTestResults,
    ) -> Promise<(), Error> {
        match self.auth {
            AuthLevel::Secure | AuthLevel::Debug => {
                let params = params.get().unwrap();
                let src = mailbox::Address::ApiCapnp { uid: self.node.uid };
                let dst = mailbox::Address::DeviceHidio { uid: self.uid };

                struct CommandInterface {
                    src: mailbox::Address,
                    dst: mailbox::Address,
                    mailbox: mailbox::Mailbox,
                    results: hidio_capnp::node::ManufacturingTestResults,
                }
                impl
                    Commands<
                        { mailbox::HIDIO_PKT_BUF_DATA_SIZE },
                        { mailbox::HIDIO_PKT_BUF_DATA_SIZE - 1 },
                        { mailbox::HIDIO_PKT_BUF_DATA_SIZE - 2 },
                        { mailbox::HIDIO_PKT_BUF_DATA_SIZE - 4 },
                        1, // TODO(HaaTa): https://github.com/japaric/heapless/issues/252
                    > for CommandInterface
                {
                    fn tx_packetbuffer_send(
                        &mut self,
                        buf: &mut mailbox::HidIoPacketBuffer,
                    ) -> Result<(), CommandError> {
                        if let Some(rcvmsg) = self.mailbox.try_send_message(mailbox::Message {
                            src: self.src,
                            dst: self.dst,
                            data: buf.clone(),
                        })? {
                            // Handle ack/nak
                            self.rx_message_handling(rcvmsg.data)?;
                        }
                        Ok(())
                    }
                    fn h0050_manufacturing_ack(
                        &mut self,
                        _data: h0050::Ack,
                    ) -> Result<(), CommandError> {
                        let status = self.results.get().init_status();
                        status.init_success();
                        Ok(())
                    }
                    fn h0050_manufacturing_nak(
                        &mut self,
                        _data: h0050::Nak,
                    ) -> Result<(), CommandError> {
                        let status = self.results.get().init_status();
                        status.init_error();
                        Ok(())
                    }
                }
                let mut intf = CommandInterface {
                    src,
                    dst,
                    mailbox: self.mailbox.clone(),
                    results,
                };

                // Lookup command
                let command = match params.get_command().unwrap().get_command().unwrap() {
                    hidio_capnp::node::manufacturing::Command::LedTestSequence => {
                        h0050::Command::LedTestSequence
                    }
                    hidio_capnp::node::manufacturing::Command::LedCycleKeypressTest => {
                        h0050::Command::LedCycleKeypressTest
                    }
                    hidio_capnp::node::manufacturing::Command::HallEffectSensorTest => {
                        h0050::Command::HallEffectSensorTest
                    }
                };
                let argument = match params.get_command().unwrap().which().unwrap() {
                    hidio_capnp::node::manufacturing::Which::LedTestSequence(val) => {
                        h0050::Argument {
                            led_test_sequence: match val.unwrap() {
                                hidio_capnp::node::manufacturing::LedTestSequenceArg::Disable => {
                                    h0050::args::LedTestSequence::Disable
                                }
                                hidio_capnp::node::manufacturing::LedTestSequenceArg::Enable => {
                                    h0050::args::LedTestSequence::Enable
                                }
                                hidio_capnp::node::manufacturing::LedTestSequenceArg::ActivateLedShortTest => {
                                    h0050::args::LedTestSequence::ActivateLedShortTest
                                }
                                hidio_capnp::node::manufacturing::LedTestSequenceArg::ActivateLedOpenCircuitTest => {
                                    h0050::args::LedTestSequence::ActivateLedOpenCircuitTest
                                }
                            },
                        }
                    }
                    hidio_capnp::node::manufacturing::Which::LedCycleKeypressTest(val) => {
                        h0050::Argument {
                            led_cycle_keypress_test: match val.unwrap() {
                                hidio_capnp::node::manufacturing::LedCycleKeypressTestArg::Disable => {
                                    h0050::args::LedCycleKeypressTest::Disable
                                }
                                hidio_capnp::node::manufacturing::LedCycleKeypressTestArg::Enable => {
                                    h0050::args::LedCycleKeypressTest::Enable
                                }
                            },
                        }
                    }
                    hidio_capnp::node::manufacturing::Which::HallEffectSensorTest(val) => {
                        h0050::Argument {
                            hall_effect_sensor_test: match val.unwrap() {
                                hidio_capnp::node::manufacturing::HallEffectSensorTestArg::DisableAll => {
                                    h0050::args::HallEffectSensorTest::DisableAll
                                }
                                hidio_capnp::node::manufacturing::HallEffectSensorTestArg::PassFailTestToggle => {
                                    h0050::args::HallEffectSensorTest::PassFailTestToggle
                                }
                                hidio_capnp::node::manufacturing::HallEffectSensorTestArg::LevelCheckToggle => {
                                    h0050::args::HallEffectSensorTest::LevelCheckToggle
                                }
                            },
                        }
                    }
                };

                // Send command
                if let Err(e) = intf.h0050_manufacturing(h0050::Cmd { command, argument }) {
                    return Promise::err(capnp::Error {
                        kind: ::capnp::ErrorKind::Failed,
                        description: format!("Error (manufacturing_test): {:?}", e),
                    });
                }
                Promise::ok(())
            }
            _ => Promise::err(capnp::Error {
                kind: ::capnp::ErrorKind::Failed,
                description: "Insufficient authorization level".to_string(),
            }),
        }
    }

    fn pixel_set(
        &mut self,
        params: hidio_capnp::node::PixelSetParams,
        mut results: hidio_capnp::node::PixelSetResults,
    ) -> Promise<(), Error> {
        let src = mailbox::Address::ApiCapnp { uid: self.node.uid };
        let dst = mailbox::Address::DeviceHidio { uid: self.uid };

        struct CommandInterface {
            src: mailbox::Address,
            dst: mailbox::Address,
            mailbox: mailbox::Mailbox,
        }
        impl
            Commands<
                { mailbox::HIDIO_PKT_BUF_DATA_SIZE },
                { mailbox::HIDIO_PKT_BUF_DATA_SIZE - 1 },
                { mailbox::HIDIO_PKT_BUF_DATA_SIZE - 2 },
                { mailbox::HIDIO_PKT_BUF_DATA_SIZE - 4 },
                1, // TODO(HaaTa): https://github.com/japaric/heapless/issues/252
            > for CommandInterface
        {
            fn tx_packetbuffer_send(
                &mut self,
                buf: &mut mailbox::HidIoPacketBuffer,
            ) -> Result<(), CommandError> {
                if let Some(rcvmsg) = self.mailbox.try_send_message(mailbox::Message {
                    src: self.src,
                    dst: self.dst,
                    data: buf.clone(),
                })? {
                    // Handle ack/nak
                    self.rx_message_handling(rcvmsg.data)?;
                }
                Ok(())
            }

            fn h0026_directset_ack(&mut self, _data: h0026::Ack) -> Result<(), CommandError> {
                Ok(())
            }
        }
        let mut intf = CommandInterface {
            src,
            dst,
            mailbox: self.mailbox.clone(),
        };

        let params = params.get().unwrap();
        let start_address: u16 = params.get_command().unwrap().get_start_address();
        let status = results.get().init_status();

        match params.get_command().unwrap().get_type().unwrap() {
            hidio_capnp::node::pixel_set::Type::DirectSet => {
                let data = match heapless::Vec::from_slice(
                    params.get_command().unwrap().get_direct_set_data().unwrap(),
                ) {
                    Ok(data) => data,
                    Err(e) => {
                        error!("Error (pixel_set - directset - data): {:?}", e);
                        status.init_error();
                        return Promise::ok(());
                    }
                };
                if let Err(e) = intf.h0026_directset(
                    h0026::Cmd {
                        start_address,
                        data,
                    },
                    true,
                ) {
                    error!("Error (pixel_set - directset): {:?}", e);
                    status.init_error();
                    return Promise::ok(());
                }
            }
        }

        status.init_success();
        Promise::ok(())
    }

    fn pixel_setting(
        &mut self,
        params: hidio_capnp::node::PixelSettingParams,
        mut results: hidio_capnp::node::PixelSettingResults,
    ) -> Promise<(), Error> {
        let src = mailbox::Address::ApiCapnp { uid: self.node.uid };
        let dst = mailbox::Address::DeviceHidio { uid: self.uid };

        struct CommandInterface {
            src: mailbox::Address,
            dst: mailbox::Address,
            mailbox: mailbox::Mailbox,
        }
        impl
            Commands<
                { mailbox::HIDIO_PKT_BUF_DATA_SIZE },
                { mailbox::HIDIO_PKT_BUF_DATA_SIZE - 1 },
                { mailbox::HIDIO_PKT_BUF_DATA_SIZE - 2 },
                { mailbox::HIDIO_PKT_BUF_DATA_SIZE - 4 },
                1, // TODO(HaaTa): https://github.com/japaric/heapless/issues/252
            > for CommandInterface
        {
            fn tx_packetbuffer_send(
                &mut self,
                buf: &mut mailbox::HidIoPacketBuffer,
            ) -> Result<(), CommandError> {
                if let Some(rcvmsg) = self.mailbox.try_send_message(mailbox::Message {
                    src: self.src,
                    dst: self.dst,
                    data: buf.clone(),
                })? {
                    // Handle ack/nak
                    self.rx_message_handling(rcvmsg.data)?;
                }
                Ok(())
            }

            fn h0021_pixelsetting_ack(&mut self, _data: h0021::Ack) -> Result<(), CommandError> {
                Ok(())
            }
        }
        let mut intf = CommandInterface {
            src,
            dst,
            mailbox: self.mailbox.clone(),
        };

        let params = params.get().unwrap();
        let command = match params.get_command().unwrap().get_command().unwrap() {
            hidio_capnp::node::pixel_setting::Command::Control => h0021::Command::Control,
            hidio_capnp::node::pixel_setting::Command::Reset => h0021::Command::Reset,
            hidio_capnp::node::pixel_setting::Command::Clear => h0021::Command::Clear,
            hidio_capnp::node::pixel_setting::Command::Frame => h0021::Command::Frame,
        };
        let argument = match params.get_command().unwrap().which().unwrap() {
            hidio_capnp::node::pixel_setting::Which::Control(val) => h0021::Argument {
                control: match val.unwrap() {
                    hidio_capnp::node::pixel_setting::ControlArg::Disable => {
                        h0021::args::Control::Disable
                    }
                    hidio_capnp::node::pixel_setting::ControlArg::EnableStart => {
                        h0021::args::Control::EnableStart
                    }
                    hidio_capnp::node::pixel_setting::ControlArg::EnablePause => {
                        h0021::args::Control::EnablePause
                    }
                },
            },
            hidio_capnp::node::pixel_setting::Which::Reset(val) => h0021::Argument {
                reset: match val.unwrap() {
                    hidio_capnp::node::pixel_setting::ResetArg::SoftReset => {
                        h0021::args::Reset::SoftReset
                    }
                    hidio_capnp::node::pixel_setting::ResetArg::HardReset => {
                        h0021::args::Reset::HardReset
                    }
                },
            },
            hidio_capnp::node::pixel_setting::Which::Clear(val) => h0021::Argument {
                clear: match val.unwrap() {
                    hidio_capnp::node::pixel_setting::ClearArg::Clear => h0021::args::Clear::Clear,
                },
            },
            hidio_capnp::node::pixel_setting::Which::Frame(val) => h0021::Argument {
                frame: match val.unwrap() {
                    hidio_capnp::node::pixel_setting::FrameArg::NextFrame => {
                        h0021::args::Frame::NextFrame
                    }
                },
            },
        };

        let status = results.get().init_status();
        if let Err(e) = intf.h0021_pixelsetting(h0021::Cmd { command, argument }, true) {
            status.init_error();
            error!("Error (pixel_setting): {:?}", e);
            return Promise::ok(());
        }
        status.init_success();
        Promise::ok(())
    }

    fn info(
        &mut self,
        _params: hidio_capnp::node::InfoParams,
        mut results: hidio_capnp::node::InfoResults,
    ) -> Promise<(), Error> {
        let src = mailbox::Address::ApiCapnp { uid: self.node.uid };
        let dst = mailbox::Address::DeviceHidio { uid: self.uid };

        struct CommandInterface {
            src: mailbox::Address,
            dst: mailbox::Address,
            mailbox: mailbox::Mailbox,
            results: hidio_capnp::node::InfoResults,
        }
        impl
            Commands<
                { mailbox::HIDIO_PKT_BUF_DATA_SIZE },
                { mailbox::HIDIO_PKT_BUF_DATA_SIZE - 1 },
                { mailbox::HIDIO_PKT_BUF_DATA_SIZE - 2 },
                { mailbox::HIDIO_PKT_BUF_DATA_SIZE - 4 },
                1, // TODO(HaaTa): https://github.com/japaric/heapless/issues/252
            > for CommandInterface
        {
            fn tx_packetbuffer_send(
                &mut self,
                buf: &mut mailbox::HidIoPacketBuffer,
            ) -> Result<(), CommandError> {
                if let Some(rcvmsg) = self.mailbox.try_send_message(mailbox::Message {
                    src: self.src,
                    dst: self.dst,
                    data: buf.clone(),
                })? {
                    // Handle ack/nak
                    self.rx_message_handling(rcvmsg.data)?;
                }
                Ok(())
            }

            fn h0001_info_ack(
                &mut self,
                data: h0001::Ack<{ mailbox::HIDIO_PKT_BUF_DATA_SIZE - 1 }>,
            ) -> Result<(), CommandError> {
                use h0001::Property;

                let mut info = self.results.get().get_info().unwrap();
                match data.property {
                    Property::MajorVersion => info.set_hidio_major_version(data.number),
                    Property::MinorVersion => info.set_hidio_minor_version(data.number),
                    Property::PatchVersion => info.set_hidio_patch_version(data.number),
                    Property::DeviceName => info.set_device_name(&data.string),
                    Property::DeviceSerialNumber => info.set_device_serial(&data.string),
                    Property::DeviceVersion => info.set_device_version(&data.string),
                    Property::DeviceMcu => info.set_device_mcu(&data.string),
                    Property::DeviceVendor => info.set_device_vendor(&data.string),
                    Property::FirmwareName => info.set_firmware_name(&data.string),
                    Property::FirmwareVersion => info.set_firmware_version(&data.string),
                    _ => {}
                }

                Ok(())
            }
        }
        results.get().init_info();
        let mut intf = CommandInterface {
            src,
            dst,
            mailbox: self.mailbox.clone(),
            results,
        };

        // Get version info
        let _ = intf.h0001_info(h0001::Cmd {
            property: h0001::Property::MajorVersion,
        });
        let _ = intf.h0001_info(h0001::Cmd {
            property: h0001::Property::MinorVersion,
        });
        let _ = intf.h0001_info(h0001::Cmd {
            property: h0001::Property::PatchVersion,
        });

        // Get device info
        let _ = intf.h0001_info(h0001::Cmd {
            property: h0001::Property::DeviceName,
        });
        let _ = intf.h0001_info(h0001::Cmd {
            property: h0001::Property::DeviceSerialNumber,
        });
        let _ = intf.h0001_info(h0001::Cmd {
            property: h0001::Property::DeviceVersion,
        });
        let _ = intf.h0001_info(h0001::Cmd {
            property: h0001::Property::DeviceMcu,
        });
        let _ = intf.h0001_info(h0001::Cmd {
            property: h0001::Property::DeviceVendor,
        });

        // Get firmware info
        let _ = intf.h0001_info(h0001::Cmd {
            property: h0001::Property::FirmwareName,
        });
        let _ = intf.h0001_info(h0001::Cmd {
            property: h0001::Property::FirmwareVersion,
        });
        Promise::ok(())
    }

    fn supported_ids(
        &mut self,
        _params: hidio_capnp::node::SupportedIdsParams,
        results: hidio_capnp::node::SupportedIdsResults,
    ) -> Promise<(), Error> {
        const MAX_IDS: usize = 200;
        match self.auth {
            AuthLevel::Secure | AuthLevel::Debug => {
                let src = mailbox::Address::ApiCapnp { uid: self.node.uid };
                let dst = mailbox::Address::DeviceHidio { uid: self.uid };

                struct CommandInterface {
                    src: mailbox::Address,
                    dst: mailbox::Address,
                    mailbox: mailbox::Mailbox,
                    results: hidio_capnp::node::SupportedIdsResults,
                }
                impl
                    Commands<
                        { mailbox::HIDIO_PKT_BUF_DATA_SIZE },
                        { mailbox::HIDIO_PKT_BUF_DATA_SIZE - 1 },
                        { mailbox::HIDIO_PKT_BUF_DATA_SIZE - 2 },
                        { mailbox::HIDIO_PKT_BUF_DATA_SIZE - 4 },
                        MAX_IDS,
                    > for CommandInterface
                {
                    fn tx_packetbuffer_send(
                        &mut self,
                        buf: &mut mailbox::HidIoPacketBuffer,
                    ) -> Result<(), CommandError> {
                        if let Some(rcvmsg) = self.mailbox.try_send_message(mailbox::Message {
                            src: self.src,
                            dst: self.dst,
                            data: buf.clone(),
                        })? {
                            // Handle ack/nak
                            self.rx_message_handling(rcvmsg.data)?;
                        }
                        Ok(())
                    }
                    fn h0000_supported_ids_ack(
                        &mut self,
                        data: h0000::Ack<MAX_IDS>,
                    ) -> Result<(), CommandError> {
                        self.results.get().init_ids(data.ids.len() as u32);
                        for (i, id) in data.ids.iter().enumerate() {
                            let mut entry = self.results.get().get_ids().unwrap().get(i as u32);
                            entry.set_uid(*id as u32);
                            entry.set_name(format!("{:?}", id).as_str());
                        }
                        Ok(())
                    }
                }
                let mut intf = CommandInterface {
                    src,
                    dst,
                    mailbox: self.mailbox.clone(),
                    results,
                };

                // Send command
                if let Err(e) = intf.h0000_supported_ids(h0000::Cmd {}) {
                    return Promise::err(capnp::Error {
                        kind: ::capnp::ErrorKind::Failed,
                        description: format!("Error (supported_ids): {:?}", e),
                    });
                }
                Promise::ok(())
            }
            _ => Promise::err(capnp::Error {
                kind: ::capnp::ErrorKind::Failed,
                description: "Insufficient authorization level".to_string(),
            }),
        }
    }

    fn test(
        &mut self,
        params: hidio_capnp::node::TestParams,
        results: hidio_capnp::node::TestResults,
    ) -> Promise<(), Error> {
        const MAX_DATA_SIZE: usize = 500;
        match self.auth {
            AuthLevel::Secure | AuthLevel::Debug => {
                let params = params.get().unwrap();
                let src = mailbox::Address::ApiCapnp { uid: self.node.uid };
                let dst = mailbox::Address::DeviceHidio { uid: self.uid };

                struct CommandInterface {
                    src: mailbox::Address,
                    dst: mailbox::Address,
                    mailbox: mailbox::Mailbox,
                    results: hidio_capnp::node::TestResults,
                }
                impl
                    Commands<
                        { mailbox::HIDIO_PKT_BUF_DATA_SIZE },
                        { mailbox::HIDIO_PKT_BUF_DATA_SIZE - 1 },
                        { mailbox::HIDIO_PKT_BUF_DATA_SIZE - 2 },
                        { mailbox::HIDIO_PKT_BUF_DATA_SIZE - 4 },
                        MAX_DATA_SIZE,
                    > for CommandInterface
                {
                    fn tx_packetbuffer_send(
                        &mut self,
                        buf: &mut mailbox::HidIoPacketBuffer,
                    ) -> Result<(), CommandError> {
                        if let Some(rcvmsg) = self.mailbox.try_send_message(mailbox::Message {
                            src: self.src,
                            dst: self.dst,
                            data: buf.clone(),
                        })? {
                            // Handle ack/nak
                            self.rx_message_handling(rcvmsg.data)?;
                        }
                        Ok(())
                    }
                    fn h0002_test_ack(
                        &mut self,
                        data: h0002::Ack<MAX_DATA_SIZE>,
                    ) -> Result<(), CommandError> {
                        self.results.get().init_data(data.data.len() as u32);
                        for (i, byte) in data.data.iter().enumerate() {
                            self.results.get().get_data().unwrap()[i] = *byte;
                        }
                        Ok(())
                    }
                }
                let mut intf = CommandInterface {
                    src,
                    dst,
                    mailbox: self.mailbox.clone(),
                    results,
                };

                // Send command
                if let Err(e) = intf.h0002_test(
                    h0002::Cmd::<MAX_DATA_SIZE> {
                        data: heapless::Vec::from_slice(params.get_data().unwrap()).unwrap(),
                    },
                    false,
                ) {
                    return Promise::err(capnp::Error {
                        kind: ::capnp::ErrorKind::Failed,
                        description: format!("Error (supported_ids): {:?}", e),
                    });
                }
                Promise::ok(())
            }
            _ => Promise::err(capnp::Error {
                kind: ::capnp::ErrorKind::Failed,
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
        // First check to make sure we're actually trying to subscribe to something
        let _options = match pry!(params.get()).get_options() {
            Ok(options) => {
                if options.len() == 0 {
                    return Promise::err(capnp::Error {
                        kind: ::capnp::ErrorKind::Failed,
                        description: "No subscription options specified".to_string(),
                    });
                }
                // TODO Store/Setup options for KeyboardSubscriberHandle
                options
            }
            Err(e) => {
                return Promise::err(capnp::Error {
                    kind: ::capnp::ErrorKind::Failed,
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

    fn unicode_text(
        &mut self,
        params: daemon_capnp::daemon::UnicodeTextParams,
        mut _results: daemon_capnp::daemon::UnicodeTextResults,
    ) -> Promise<(), Error> {
        let params = params.get().unwrap();
        let string = heapless::String::from(params.get_string().unwrap());
        let src = mailbox::Address::ApiCapnp { uid: self.node.uid };
        let dst = mailbox::Address::Module;

        struct CommandInterface {
            src: mailbox::Address,
            dst: mailbox::Address,
            mailbox: mailbox::Mailbox,
            result: Result<h0017::Ack, h0017::Nak>,
        }
        impl
            Commands<
                { mailbox::HIDIO_PKT_BUF_DATA_SIZE },
                { mailbox::HIDIO_PKT_BUF_DATA_SIZE - 1 },
                { mailbox::HIDIO_PKT_BUF_DATA_SIZE - 2 },
                { mailbox::HIDIO_PKT_BUF_DATA_SIZE - 4 },
                1, // TODO(HaaTa): https://github.com/japaric/heapless/issues/252
            > for CommandInterface
        {
            fn tx_packetbuffer_send(
                &mut self,
                buf: &mut mailbox::HidIoPacketBuffer,
            ) -> Result<(), CommandError> {
                if let Some(rcvmsg) = self.mailbox.try_send_message(mailbox::Message {
                    src: self.src,
                    dst: self.dst,
                    data: buf.clone(),
                })? {
                    // Handle ack/nak
                    self.rx_message_handling(rcvmsg.data)?;
                }
                Ok(())
            }
            fn h0017_unicodetext_ack(&mut self, data: h0017::Ack) -> Result<(), CommandError> {
                self.result = Ok(data);
                Ok(())
            }
            fn h0017_unicodetext_nak(&mut self, data: h0017::Nak) -> Result<(), CommandError> {
                self.result = Err(data);
                Ok(())
            }
        }
        let mut intf = CommandInterface {
            src,
            dst,
            mailbox: self.mailbox.clone(),
            result: Err(h0017::Nak {}),
        };

        // Send command
        let cmd = h0017::Cmd { string };
        if let Err(e) = intf.h0017_unicodetext(cmd.clone(), false) {
            return Promise::err(capnp::Error {
                kind: ::capnp::ErrorKind::Failed,
                description: format!("Error (unicodetext): {:?} {:?}", cmd, e),
            });
        }

        // Wait for Ack/Nak
        match intf.result {
            Ok(_msg) => Promise::ok(()),
            Err(msg) => Promise::err(capnp::Error {
                kind: ::capnp::ErrorKind::Failed,
                description: format!("Error (unicode_text): {:?}", msg),
            }),
        }
    }

    fn unicode_state(
        &mut self,
        params: daemon_capnp::daemon::UnicodeStateParams,
        mut _results: daemon_capnp::daemon::UnicodeStateResults,
    ) -> Promise<(), Error> {
        let params = params.get().unwrap();
        let symbols = heapless::String::from(params.get_characters().unwrap());
        let src = mailbox::Address::ApiCapnp { uid: self.node.uid };
        let dst = mailbox::Address::Module;

        struct CommandInterface {
            src: mailbox::Address,
            dst: mailbox::Address,
            mailbox: mailbox::Mailbox,
            result: Result<h0018::Ack, h0018::Nak>,
        }
        impl
            Commands<
                { mailbox::HIDIO_PKT_BUF_DATA_SIZE },
                { mailbox::HIDIO_PKT_BUF_DATA_SIZE - 1 },
                { mailbox::HIDIO_PKT_BUF_DATA_SIZE - 2 },
                { mailbox::HIDIO_PKT_BUF_DATA_SIZE - 4 },
                1, // TODO(HaaTa): https://github.com/japaric/heapless/issues/252
            > for CommandInterface
        {
            fn tx_packetbuffer_send(
                &mut self,
                buf: &mut mailbox::HidIoPacketBuffer,
            ) -> Result<(), CommandError> {
                if let Some(rcvmsg) = self.mailbox.try_send_message(mailbox::Message {
                    src: self.src,
                    dst: self.dst,
                    data: buf.clone(),
                })? {
                    // Handle ack/nak
                    self.rx_message_handling(rcvmsg.data)?;
                }
                Ok(())
            }
            fn h0018_unicodestate_ack(&mut self, data: h0018::Ack) -> Result<(), CommandError> {
                self.result = Ok(data);
                Ok(())
            }
            fn h0018_unicodestate_nak(&mut self, data: h0018::Nak) -> Result<(), CommandError> {
                self.result = Err(data);
                Ok(())
            }
        }
        let mut intf = CommandInterface {
            src,
            dst,
            mailbox: self.mailbox.clone(),
            result: Err(h0018::Nak {}),
        };

        // Send command
        let cmd = h0018::Cmd { symbols };
        if let Err(e) = intf.h0018_unicodestate(cmd.clone(), false) {
            return Promise::err(capnp::Error {
                kind: ::capnp::ErrorKind::Failed,
                description: format!("Error (unicodetext): {:?} {:?}", cmd, e),
            });
        }

        // Wait for Ack/Nak
        match intf.result {
            Ok(_msg) => Promise::ok(()),
            Err(msg) => Promise::err(capnp::Error {
                kind: ::capnp::ErrorKind::Failed,
                description: format!("Error (unicode_text): {:?}", msg),
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
    trace!("Building address");
    let addr = LISTEN_ADDR
        .to_socket_addrs()?
        .next()
        .expect("could not parse address");
    trace!("Address: {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    println!("API: Listening on {}", addr);

    // Generate new self-signed public/private key
    // Private key is not written to disk and generated each time
    let subject_alt_names = vec!["localhost".to_string()];
    let pair = generate_simple_self_signed(subject_alt_names).unwrap();

    let cert = Certificate(pair.serialize_der().unwrap());
    let pkey = PrivateKey(pair.serialize_private_key_der());
    let config = ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(vec![cert], pkey)
        .unwrap();
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
        trace!("S1");
        let stream_abortable =
            futures::future::Abortable::new(listener.accept(), abort_registration);
        trace!("S2");
        let (stream, _addr) = stream_abortable.await??;
        trace!("S3");
        stream.set_nodelay(true)?;
        let acceptor = acceptor.clone();
        trace!("S4");

        // Make sure to timeout if no https handshake is attempted
        let stream = match tokio::time::timeout(
            std::time::Duration::from_millis(100),
            acceptor.accept(stream),
        )
        .await
        {
            Ok(stream) => match stream {
                Ok(stream) => stream,
                Err(_) => {
                    continue;
                }
            },
            Err(_) => {
                continue;
            }
        };
        trace!("S5");

        // Save connection address for later
        let addr = stream.get_ref().0.peer_addr().ok().unwrap();
        trace!("S6");

        // Setup reader/writer stream pair
        let (reader, writer) = futures_util::io::AsyncReadExt::split(
            tokio_util::compat::TokioAsyncReadCompatExt::compat(stream),
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
            Box::pin(
                rpc_system
                    .map_err(|e| info!("rpc_system: {}", e))
                    .map(move |_| {
                        info!("Connection closed:7185 - {:?} - uid:{}", addr, uid);

                        // Client disconnected, delete node
                        let connected_nodes = connections.read().unwrap()[&uid].clone();
                        nodes
                            .write()
                            .unwrap()
                            .retain(|x| !connected_nodes.contains(&x.uid));
                    }),
            )
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
                let stream = BroadcastStream::new(receiver)
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
                let stream = BroadcastStream::new(receiver)
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

            // TODO Handle filtering based on what has been registered
            // Filters
            //  cli output
            //  host macro (TODO)
            //  kll trigger (TODO)
            //  layer (TODO)
            let mut stream = stream.filter(|msg| {
                (msg.data.ptype == HidIoPacketType::Data
                    || msg.data.ptype == HidIoPacketType::NaData)
                    && (msg.data.id == HidIoCommandId::TerminalOut
                        || msg.data.id == HidIoCommandId::KllState
                        || msg.data.id == HidIoCommandId::HostMacro
                        || msg.data.id == HidIoCommandId::ManufacturingResult)
            });

            // Handle stream
            while let Some(msg) = stream.next().await {
                let src = msg.src;
                let dst = msg.dst;

                struct CommandInterface {
                    src: mailbox::Address,
                    dst: mailbox::Address,
                    mailbox: mailbox::Mailbox,
                    request: ::capnp::capability::Request<
                        keyboard_capnp::keyboard::subscriber::update_params::Owned,
                        keyboard_capnp::keyboard::subscriber::update_results::Owned,
                    >,
                }
                impl
                    Commands<
                        { mailbox::HIDIO_PKT_BUF_DATA_SIZE },
                        { mailbox::HIDIO_PKT_BUF_DATA_SIZE - 1 },
                        { mailbox::HIDIO_PKT_BUF_DATA_SIZE - 2 },
                        { mailbox::HIDIO_PKT_BUF_DATA_SIZE - 4 },
                        1, // TODO(HaaTa): https://github.com/japaric/heapless/issues/252
                    > for CommandInterface
                {
                    fn tx_packetbuffer_send(
                        &mut self,
                        buf: &mut mailbox::HidIoPacketBuffer,
                    ) -> Result<(), CommandError> {
                        if let Some(rcvmsg) = self.mailbox.try_send_message(mailbox::Message {
                            src: self.src,
                            dst: self.dst,
                            data: buf.clone(),
                        })? {
                            // Handle ack/nak
                            self.rx_message_handling(rcvmsg.data)?;
                        }
                        Ok(())
                    }
                    fn h0034_terminalout_cmd(
                        &mut self,
                        data: h0034::Cmd<{ mailbox::HIDIO_PKT_BUF_DATA_SIZE }>,
                    ) -> Result<h0034::Ack, h0034::Nak> {
                        // Build Signal message
                        let mut signal = self.request.get().init_signal();
                        signal.set_time(
                            std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .expect("Time went backwards")
                                .as_millis() as u64,
                        );
                        signal.init_data().init_cli().set_output(&data.output);

                        Ok(h0034::Ack {})
                    }
                    fn h0034_terminalout_nacmd(
                        &mut self,
                        data: h0034::Cmd<{ mailbox::HIDIO_PKT_BUF_DATA_SIZE }>,
                    ) -> Result<(), CommandError> {
                        // Build Signal message
                        let mut signal = self.request.get().init_signal();
                        signal.set_time(
                            std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .expect("Time went backwards")
                                .as_millis() as u64,
                        );
                        signal.init_data().init_cli().set_output(&data.output);

                        Ok(())
                    }
                    fn h0051_manufacturingres_cmd(
                        &mut self,
                        data: h0051::Cmd<{ mailbox::HIDIO_PKT_BUF_DATA_SIZE - 4 }>,
                    ) -> Result<h0051::Ack, h0051::Nak> {
                        // Build Signal message
                        let mut signal = self.request.get().init_signal();
                        signal.set_time(
                            std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .expect("Time went backwards")
                                .as_millis() as u64,
                        );
                        let mut result = signal.init_data().init_manufacturing();
                        let command = match data.command {
                            h0051::Command::LedTestSequence => {
                                keyboard_capnp::keyboard::signal::manufacturing_result::Command::LedTestSequence
                            }
                            h0051::Command::LedCycleKeypressTest => {
                                keyboard_capnp::keyboard::signal::manufacturing_result::Command::LedCycleKeypressTest
                            }
                            h0051::Command::HallEffectSensorTest => {
                                keyboard_capnp::keyboard::signal::manufacturing_result::Command::HallEffectSensorTest
                            }
                            _ => {
                                return Err(h0051::Nak {});
                            }
                        };
                        result.set_cmd(command);
                        result.set_arg(unsafe { data.argument.raw });
                        let mut result = result.init_data(data.data.len() as u32);
                        for (i, f) in data.data.iter().enumerate() {
                            result.set(i as u32, *f);
                        }
                        Ok(h0051::Ack {})
                    }
                }

                // Setup interface
                let mut intf = CommandInterface {
                    src,
                    dst,
                    mailbox: mailbox.clone(),
                    // Forward message to api callback
                    request: subscriptions
                        .read()
                        .unwrap()
                        .keyboard_node
                        .subscribers
                        .get(&last_keyboard_next_id)
                        .unwrap()
                        .client
                        .update_request(),
                };

                // Process incoming message
                // TODO(HaaTa): Determine the best way to return some sort of error (capnp) if this fails
                if let Err(err) = intf.rx_message_handling(msg.data) {
                    error!("rx_message_handling failed!: {:?}", err);
                }

                // Block on each send, drop subscription on failure
                if let Err(e) = intf.request.send().promise.await {
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
                let stream = BroadcastStream::new(receiver)
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
                    HidIoPacketType::NaData => hidio_capnp::hid_io::packet::Type::NaData,
                    HidIoPacketType::Ack => hidio_capnp::hid_io::packet::Type::Ack,
                    HidIoPacketType::Nak => hidio_capnp::hid_io::packet::Type::Nak,
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
                        #[allow(clippy::significant_drop_in_scrutinee)]
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
pub fn supported_ids() -> Vec<HidIoCommandId> {
    vec![
        HidIoCommandId::DirectSet,
        HidIoCommandId::FlashMode,
        HidIoCommandId::GetInfo,
        HidIoCommandId::HostMacro,
        HidIoCommandId::KllState,
        HidIoCommandId::ManufacturingResult,
        HidIoCommandId::ManufacturingTest,
        HidIoCommandId::PixelSetting,
        HidIoCommandId::SleepMode,
        HidIoCommandId::SupportedIds,
        HidIoCommandId::TerminalCmd,
        HidIoCommandId::TerminalOut,
        HidIoCommandId::TestPacket,
    ]
}

/// Cap'n'Proto API Initialization
/// Sets up a localhost socket to deal with localhost-only API usages
/// Some API usages may require external authentication to validate trustworthiness
#[cfg(feature = "api")]
pub async fn initialize(mailbox: mailbox::Mailbox) {
    info!("Initializing api...");
    let rt = mailbox.clone().rt;

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

#[cfg(not(feature = "api"))]
pub async fn initialize(_mailbox: mailbox::Mailbox) {}
