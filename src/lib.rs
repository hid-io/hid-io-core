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

#![feature(drain_filter)]
#![feature(allow_fail)]

// ----- Crates -----

#[macro_use]
extern crate log;

use std::sync::atomic::Ordering;

// ----- Modules -----

/// capnp interface for other programs to hook into
pub mod api;

/// communication with hidapi compatable devices
pub mod device;

/// logging functions
pub mod logging;

/// mpmc mailbox implementation for hid-io-core (e.g. packet broadcast with filters)
pub mod mailbox;

/// built-in features and command handlers
pub mod module;

/// parsing for different data types
pub mod protocol;

/// Compile time information
pub mod built_info {
    // This file is generated at build time using build.rs
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

/// [AUTO GENERATED]
#[cfg(feature = "api")]
pub mod common_capnp {
    #![allow(clippy::all)]
    include!(concat!(env!("OUT_DIR"), "/common_capnp.rs"));
}

/// [AUTO GENERATED]
#[cfg(feature = "api")]
pub mod daemon_capnp {
    #![allow(clippy::all)]
    include!(concat!(env!("OUT_DIR"), "/daemon_capnp.rs"));
}

/// [AUTO GENERATED]
#[cfg(feature = "api")]
pub mod hidio_capnp {
    #![allow(clippy::all)]
    include!(concat!(env!("OUT_DIR"), "/hidio_capnp.rs"));
}

/// [AUTO GENERATED]
#[cfg(feature = "api")]
pub mod keyboard_capnp {
    #![allow(clippy::all)]
    include!(concat!(env!("OUT_DIR"), "/keyboard_capnp.rs"));
}

// ----- Functions -----

use lazy_static::lazy_static;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

lazy_static! {
    /// Any thread can set this to false to signal shutdown
    pub static ref RUNNING: Arc<AtomicBool> = Arc::new(AtomicBool::new(true));
}

/// Main entry-point for the hid-io-core library
pub async fn initialize(
    rt: Arc<tokio::runtime::Runtime>,
    mailbox: mailbox::Mailbox,
) -> Result<(), std::io::Error> {
    // Setup signal handler
    let r = RUNNING.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");
    println!("Press Ctrl-C to exit...");

    // Wait until completion
    let (_, _, _) = tokio::join!(
        // Initialize Modules
        module::initialize(rt.clone(), mailbox.clone()),
        // Initialize Device monitoring
        device::initialize(rt.clone(), mailbox.clone()),
        // Initialize Cap'n'Proto API Server
        api::initialize(rt.clone(), mailbox),
    );
    Ok(())
}
