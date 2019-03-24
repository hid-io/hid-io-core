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

#![feature(drain_filter)]

// ----- Crates -----

#[macro_use]
extern crate log;

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

use lazy_static::lazy_static;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

lazy_static! {
    // Any thread can set this to false to signal shutdown
    pub static ref RUNNING: Arc<AtomicBool> = Arc::new(AtomicBool::new(true));
}
