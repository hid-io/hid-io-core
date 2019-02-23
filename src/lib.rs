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

#![feature(await_macro, async_await, futures_api)]
#![feature(drain_filter)]

// ----- Crates -----

#[macro_use]
extern crate log;

//extern crate tokio;

// ----- Modules -----

pub mod api;
pub mod device;
pub mod module;
pub mod protocol;

pub mod built_info {
    // This file is generated at build time using build.rs
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

pub mod common_capnp {
    #![allow(clippy::all)]
    include!(concat!(env!("OUT_DIR"), "/common_capnp.rs"));
}

pub mod devicefunction_capnp {
    #![allow(clippy::all)]
    include!(concat!(env!("OUT_DIR"), "/devicefunction_capnp.rs"));
}

pub mod hidiowatcher_capnp {
    #![allow(clippy::all)]
    include!(concat!(env!("OUT_DIR"), "/hidiowatcher_capnp.rs"));
}

pub mod hidio_capnp {
    #![allow(clippy::all)]
    include!(concat!(env!("OUT_DIR"), "/hidio_capnp.rs"));
}

pub mod hostmacro_capnp {
    #![allow(clippy::all)]
    include!(concat!(env!("OUT_DIR"), "/hostmacro_capnp.rs"));
}

pub mod usbkeyboard_capnp {
    #![allow(clippy::all)]
    include!(concat!(env!("OUT_DIR"), "/usbkeyboard_capnp.rs"));
}

// ----- Functions -----

use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use lazy_static::lazy_static;
lazy_static! {
    pub static ref RUNNING: Arc<AtomicBool> = Arc::new(AtomicBool::new(true));
}

/*use crate::device::hidusb::HIDIOMailbox;
use crate::device::hidusb::HIDIOMailer;
use crate::device::hidusb::HIDIOMessage;

use std::sync::mpsc;
use std::sync::mpsc::channel;
struct MailSystem {
    incoming: (mpsc::Sender<HIDIOMessage>, mpsc::Receiver<HIDIOMessage>),
    outgoing: (mpsc::Sender<HIDIOMessage>, mpsc::Receiver<HIDIOMessage>),
}

impl MailSystem {
	pub fn new() -> MailSystem {
		MailSystem {
			incoming: channel::<HIDIOMessage>(),
			outgoing: channel::<HIDIOMessage>(),
		}
	}

	pub fn create_mailbox(&self) -> HIDIOMailbox {
	    let rx = &self.outgoing.1;
	    let tx = self.incoming.0.clone();
	    HIDIOMailbox::new(rx, tx)
	}

	pub fn create_mailer(&self) -> HIDIOMailer {
	    let rx = &self.incoming.1;
	    let tx = self.outgoing.0.clone();
	    HIDIOMailer::new(rx, tx)
	}
}*/

/*
let (incoming_tx, incoming_rx) = channel::<HIDIOMessage>();
let (outgoing_tx, outgoing_rx) = channel::<HIDIOMessage>();
let mailbox = HIDIOMailbox::new(outgoing_rx, incoming_tx);
let mut mailer = HIDIOMailer::new(incoming_rx, outgoing_tx);*/
