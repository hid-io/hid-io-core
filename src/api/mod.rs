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
use capnp_rpc::{rpc_twoparty_capnp, twoparty, RpcSystem};
use tokio::io::AsyncRead;
use tokio::prelude::{Future, Stream};
use tokio::runtime::current_thread;

use std::fs;
use std::io::BufReader;
use std::sync::Arc;
use tokio::prelude::future::ok;
use tokio_rustls::{
    rustls::{AllowAnyAuthenticatedClient, RootCertStore, ServerConfig},
    TlsAcceptor,
};

const USE_SSL: bool = false;

// ----- Functions -----

enum AuthLevel {
    Basic,
    Secure,
    Debug,
}

use crate::hidio_capnp::h_i_d_i_o_server::*;

struct HIDIOServerImpl;

impl HIDIOServerImpl {
    fn new() -> HIDIOServerImpl {
        HIDIOServerImpl {}
    }
}

impl h_i_d_i_o_server::Server for HIDIOServerImpl {
    fn basic(&mut self, _params: BasicParams, mut results: BasicResults) -> Promise<(), Error> {
        results.get().set_port(
            h_i_d_i_o::ToClient::new(HIDIOImpl::new(AuthLevel::Basic))
                .into_client::<::capnp_rpc::Server>(),
        );
        Promise::ok(())
    }

    fn auth(&mut self, _params: AuthParams, mut results: AuthResults) -> Promise<(), Error> {
        use crate::api::auth::UAC;

        // TODO: Auth implementation selection
        let authenticator = auth::DummyAuth {};

        if authenticator.auth() {
            results.get().set_port(
                h_i_d_i_o::ToClient::new(HIDIOImpl::new(AuthLevel::Secure))
                    .into_client::<::capnp_rpc::Server>(),
            );
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
    auth: AuthLevel,
}

impl HIDIOImpl {
    fn new(auth: AuthLevel) -> HIDIOImpl {
        HIDIOImpl { auth }
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

    fn init_nodes(mut nodes: capnp::struct_list::Builder<'_, destination::Owned>) {
        {
            let mut usbkbd = nodes.reborrow().get(0);
            usbkbd.set_type(NodeType::UsbKeyboard);
            usbkbd.set_name("Test Keyboard");
            usbkbd.set_serial("1467");
            usbkbd.set_id(78500);
            usbkbd.set_node(
                h_i_d_i_o_node::ToClient::new(HIDIONodeImpl::USBKeyboard(false))
                    .into_client::<::capnp_rpc::Server>(),
            );
        }
        {
            let mut hostmacro = nodes.get(1);
            hostmacro.set_type(NodeType::HidioScript);
            hostmacro.set_name("Test Script");
            hostmacro.set_serial("A&d342");
            hostmacro.set_id(99382569);
            hostmacro.set_node(
                h_i_d_i_o_node::ToClient::new(HIDIONodeImpl::HostIO(false))
                    .into_client::<::capnp_rpc::Server>(),
            );
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
        let nodes = results.get().init_nodes(2);
        HIDIOImpl::init_nodes(nodes);
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

    let hidio_server = h_i_d_i_o_server::ToClient::new(HIDIOServerImpl::new())
        .into_client::<::capnp_rpc::Server>();
    let done = connections.for_each(|connect_promise| {
        connect_promise.and_then(|socket| {
            let (reader, writer) = socket.split();

            let network = twoparty::VatNetwork::new(
                reader,
                writer,
                rpc_twoparty_capnp::Side::Server,
                Default::default(),
            );
            let rpc_system = RpcSystem::new(Box::new(network), Some(hidio_server.clone().client));
            current_thread::spawn(rpc_system.map_err(|e| println!("error: {:?}", e)));
            Ok(())
        })
    });

    current_thread::block_on_all(done).unwrap();
}
