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

#![feature(drain_filter)]

// ----- Crates -----

#[macro_use]
extern crate log;

// ----- Modules -----

/// capnp interface for other programs to hook into
pub mod api;

/// communication with hidapi compatable devices
pub mod device;

/// built-in features and command handlers
pub mod module;

/// parsing for different data types
pub mod protocol;

/// Compile time information
pub mod built_info {
    // This file is generated at build time using build.rs
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

/// [AUTO GENRATED]
pub mod blekeyboard_capnp {
    #![allow(clippy::all)]
    include!(concat!(env!("OUT_DIR"), "/blekeyboard_capnp.rs"));
}

/// [AUTO GENRATED]
pub mod blemouse_capnp {
    #![allow(clippy::all)]
    include!(concat!(env!("OUT_DIR"), "/blemouse_capnp.rs"));
}

/// [AUTO GENRATED]
pub mod common_capnp {
    #![allow(clippy::all)]
    include!(concat!(env!("OUT_DIR"), "/common_capnp.rs"));
}

/// [AUTO GENRATED]
pub mod devicefunction_capnp {
    #![allow(clippy::all)]
    include!(concat!(env!("OUT_DIR"), "/devicefunction_capnp.rs"));
}

/// [AUTO GENRATED]
pub mod hid_capnp {
    #![allow(clippy::all)]
    include!(concat!(env!("OUT_DIR"), "/hid_capnp.rs"));
}

/// [AUTO GENRATED]
pub mod hidio_capnp {
    #![allow(clippy::all)]
    include!(concat!(env!("OUT_DIR"), "/hidio_capnp.rs"));
}

/// [AUTO GENRATED]
pub mod hidiowatcher_capnp {
    #![allow(clippy::all)]
    include!(concat!(env!("OUT_DIR"), "/hidiowatcher_capnp.rs"));
}

/// [AUTO GENRATED]
pub mod hostmacro_capnp {
    #![allow(clippy::all)]
    include!(concat!(env!("OUT_DIR"), "/hostmacro_capnp.rs"));
}

/// [AUTO GENRATED]
pub mod usb_capnp {
    #![allow(clippy::all)]
    include!(concat!(env!("OUT_DIR"), "/usb_capnp.rs"));
}

/// [AUTO GENRATED]
pub mod usbkeyboard_capnp {
    #![allow(clippy::all)]
    include!(concat!(env!("OUT_DIR"), "/usbkeyboard_capnp.rs"));
}

/// [AUTO GENRATED]
pub mod usbmouse_capnp {
    #![allow(clippy::all)]
    include!(concat!(env!("OUT_DIR"), "/usbmouse_capnp.rs"));
}

// ----- Functions -----

use lazy_static::lazy_static;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

lazy_static! {
    /// Any thread can set this to false to signal shutdown
    pub static ref RUNNING: Arc<AtomicBool> = Arc::new(AtomicBool::new(true));
}
