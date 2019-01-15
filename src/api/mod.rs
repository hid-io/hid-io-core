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

/// TODO
/// capnproto server
// ----- Crates -----

// ----- Modules -----
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

// ----- Functions -----

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
            h_i_d_i_o::ToClient::new(HIDIOImpl::new()).from_server::<::capnp_rpc::Server>(),
        );
        Promise::ok(())
    }
}

use crate::hidio_capnp::h_i_d_i_o::*;

struct HIDIOImpl;

impl HIDIOImpl {
    fn new() -> HIDIOImpl {
        HIDIOImpl {}
    }

    fn init_signal(mut signal: h_i_d_i_o::signal::Builder) {
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
                    .from_server::<::capnp_rpc::Server>(),
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
                    .from_server::<::capnp_rpc::Server>(),
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

/// Cap'n'Proto API Initialization
/// Sets up a localhost socket to deal with localhost-only API usages
/// Requires TLS TODO
/// Some API usages may require external authentication to validate trustworthiness
pub fn initialize() {
    info!("Initializing api...");

    use std::net::ToSocketAddrs;
    let addr = "127.0.0.1:7185"
        .to_socket_addrs()
        .unwrap()
        .next()
        .expect("could not parse address");
    let socket = ::tokio::net::TcpListener::bind(&addr).unwrap();
    println!("API: Listening on {}", addr);

    let hidio_server = h_i_d_i_o_server::ToClient::new(HIDIOServerImpl::new())
        .from_server::<::capnp_rpc::Server>();
    let done = socket.incoming().for_each(move |socket| {
        socket.set_nodelay(true)?;
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
    });

    current_thread::block_on_all(done).unwrap();
}
