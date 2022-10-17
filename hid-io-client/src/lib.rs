/* Copyright (C) 2022 by Jacob Alexander
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in
 * all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
 * THE SOFTWARE.
 */

extern crate tokio;

use capnp_rpc::{rpc_twoparty_capnp, twoparty, Disconnector, RpcSystem};
use futures::{AsyncReadExt, FutureExt};
use hid_io_core::built_info;
use hid_io_core::common_capnp::NodeType;
use hid_io_core::hidio_capnp::{hid_io, hid_io_server};
use log::{debug, trace, warn};
use std::fs;
use std::net::ToSocketAddrs;
use std::sync::Arc;
use tokio_rustls::{rustls::ClientConfig, TlsConnector};

const LISTEN_ADDR: &str = "localhost:7185";

mod danger {
    use std::time::SystemTime;
    use tokio_rustls::rustls::{Certificate, ServerName};

    pub struct NoCertificateVerification {}

    impl rustls::client::ServerCertVerifier for NoCertificateVerification {
        fn verify_server_cert(
            &self,
            _end_entity: &Certificate,
            _intermediates: &[Certificate],
            _server_name: &ServerName,
            _scts: &mut dyn Iterator<Item = &[u8]>,
            _ocsp_response: &[u8],
            _now: SystemTime,
        ) -> Result<rustls::client::ServerCertVerified, rustls::Error> {
            Ok(rustls::client::ServerCertVerified::assertion())
        }
    }
}

pub fn format_node(node: hid_io_core::common_capnp::destination::Reader<'_>) -> String {
    format!(
        "{}: {} ({})",
        node.get_type().unwrap(),
        node.get_name().unwrap_or(""),
        node.get_serial().unwrap_or(""),
    )
}

pub enum HidioError {}

#[derive(Debug)]
pub enum AuthType {
    /// No authentication
    /// Very limited, only authentication APIs available
    None,
    /// Basic auth level (restricted API access)
    Basic,
    /// Highest auth level (full control and API access)
    Priviledged,
}

pub struct BuildInfo {
    pub pkg_version: String,
    pub git_version: String,
    pub profile: String,
    pub rust_c_version: String,
    pub host: String,
    pub target: String,
    pub built_time_utc: String,
}

pub fn lib_info() -> BuildInfo {
    let pkg_version = built_info::PKG_VERSION.to_string();
    let git_version =
        built_info::GIT_VERSION.map_or_else(|| "unknown".to_owned(), |v| format!("git {}", v));
    let profile = built_info::PROFILE.to_string();
    let rust_c_version = built_info::RUSTC_VERSION.to_string();
    let host = built_info::HOST.to_string();
    let target = built_info::TARGET.to_string();
    let built_time_utc = built_info::BUILT_TIME_UTC.to_string();

    BuildInfo {
        pkg_version,
        git_version,
        profile,
        rust_c_version,
        host,
        target,
        built_time_utc,
    }
}

pub struct HidioConnection {
    /// Internal address to hid-io-core, this is always localhost
    addr: std::net::SocketAddr,
    /// TLS connection used for hid-io-core connection
    connector: TlsConnector,
    /// TLS server name used for hid-io-core connection
    domain: rustls::ServerName,
    /// Cleanup handle for rpc_system
    rpc_disconnector: Option<Disconnector<rpc_twoparty_capnp::Side>>,
}

impl HidioConnection {
    pub fn new() -> Result<Self, ::capnp::Error> {
        let addr = LISTEN_ADDR
            .to_socket_addrs()?
            .next()
            .expect("Could not parse address");

        let config = ClientConfig::builder()
            .with_safe_defaults()
            .with_custom_certificate_verifier(Arc::new(danger::NoCertificateVerification {}))
            .with_no_client_auth();
        let connector = TlsConnector::from(Arc::new(config));

        let domain = rustls::ServerName::try_from("localhost").unwrap();

        Ok(Self {
            addr,
            connector,
            domain,
            rpc_disconnector: None,
        })
    }

    /// Async connect
    /// If retry is true, block until there is a connection.
    /// retry_delay sets the amount of time to sleep between retries
    ///
    /// Make sure to check the status of (hidio_auth, _) as you may
    /// not have successfully authenticated.
    pub async fn connect(
        &mut self,
        auth: AuthType,
        node_type: NodeType,
        name: String,
        serial_uid: String,
        retry: bool,
        retry_delay: std::time::Duration,
    ) -> Result<(Option<hid_io::Client>, hid_io_server::Client), ::capnp::Error> {
        trace!("Connecting to: {}", self.addr);
        let stream;
        loop {
            stream = match tokio::net::TcpStream::connect(self.addr).await {
                Ok(stream) => stream,
                Err(e) => {
                    if !retry {
                        return Err(::capnp::Error {
                            kind: ::capnp::ErrorKind::Failed,
                            description: format!("Failed to connect ({}): {}", self.addr, e),
                        });
                    }
                    warn!("Failed to connect ({}): {}", self.addr, e);
                    tokio::time::sleep(retry_delay).await;
                    continue;
                }
            };
            break;
        }
        stream.set_nodelay(true)?;
        let stream = self.connector.connect(self.domain.clone(), stream).await?;

        let (reader, writer) = tokio_util::compat::TokioAsyncReadCompatExt::compat(stream).split();

        let network = Box::new(twoparty::VatNetwork::new(
            reader,
            writer,
            rpc_twoparty_capnp::Side::Client,
            Default::default(),
        ));
        let mut rpc_system = RpcSystem::new(network, None);
        let hidio_server: hid_io_server::Client =
            rpc_system.bootstrap(rpc_twoparty_capnp::Side::Server);

        self.rpc_disconnector = Some(rpc_system.get_disconnector());
        let _handle = tokio::task::spawn_local(Box::pin(rpc_system.map(|_| {})));

        // Display server version information
        let request = hidio_server.version_request();
        let response = request.send().promise.await?;
        let value = response.get().unwrap().get_version().unwrap();
        debug!("Version: {}", value.get_version().unwrap());
        debug!("Buildtime: {}", value.get_buildtime().unwrap());
        debug!("Serverarch: {}", value.get_serverarch().unwrap());

        let request = hidio_server.key_request();
        let response = request.send().promise.await?;
        let value = response.get().unwrap().get_key().unwrap();
        let basic_key_path = value.get_basic_key_path().unwrap().to_string();
        let auth_key_path = value.get_auth_key_path().unwrap().to_string();
        debug!("Basic Key Path: {}", basic_key_path);
        debug!("Auth Key Path: {}", auth_key_path);

        // Lookup uid
        let uid = {
            let request = hidio_server.id_request();
            let response = request.send().promise.await?;
            let value = response.get().unwrap().get_id();
            value
        };
        debug!("Id: {}", uid);

        // Attempt to authenticate if specified
        debug!("AuthType: {:?}", auth);
        let hidio_auth = match auth {
            AuthType::None => None,
            AuthType::Basic => {
                // Attempt to read the key
                let key = fs::read_to_string(basic_key_path)?;

                // Attempt authentication
                let mut request = hidio_server.basic_request();
                let mut info = request.get().get_info()?;
                info.set_type(node_type);
                info.set_name(&name);
                info.set_serial(&serial_uid);
                info.set_id(uid);
                request.get().set_key(&key);

                Some(request.send().pipeline.get_port())
            }
            AuthType::Priviledged => {
                // Attempt to read the key
                let key = fs::read_to_string(auth_key_path)?;

                // Attempt authentication
                let mut request = hidio_server.auth_request();
                let mut info = request.get().get_info()?;
                info.set_type(node_type);
                info.set_name(&name);
                info.set_serial(&serial_uid);
                info.set_id(uid);
                request.get().set_key(&key);

                Some(request.send().pipeline.get_port())
            }
        };

        Ok((hidio_auth, hidio_server))
    }

    /// Disconnect with hid-io-core
    /// And/or stop reconnecting
    pub async fn disconnect(&mut self) -> Result<(), ::capnp::Error> {
        trace!("Disconnecting from: {}", self.addr);

        // Only await if there's something to wait for
        if let Some(rpcd) = &mut self.rpc_disconnector {
            rpcd.await?;
        }

        Ok(())
    }
}
