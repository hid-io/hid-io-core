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

#![feature(drain_filter)]

// ----- Crates -----

#[macro_use]
extern crate log;

use std::sync::atomic::Ordering;

pub use tokio;

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

pub use hid_io_protocol::HidIoCommandId;
use lazy_static::lazy_static;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

lazy_static! {
    /// Any thread can set this to false to signal shutdown
    pub static ref RUNNING: Arc<AtomicBool> = Arc::new(AtomicBool::new(true));
}

/// Supported Ids by hid-io-core
/// This is used to determine all supported ids (always recursive).
pub fn supported_ids() -> Vec<HidIoCommandId> {
    let mut ids = Vec::new();

    ids.extend(api::supported_ids().iter().cloned());
    ids.extend(device::supported_ids(true).iter().cloned());
    ids.extend(module::supported_ids(true).iter().cloned());

    // Sort, then deduplicate
    ids.sort_unstable();
    ids.dedup();

    ids
}

/// Main entry-point for the hid-io-core library
pub async fn initialize(mailbox: mailbox::Mailbox) -> Result<(), std::io::Error> {
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
        module::initialize(mailbox.clone()),
        // Initialize Device monitoring
        device::initialize(mailbox.clone()),
        // Initialize Cap'n'Proto API Server
        api::initialize(mailbox),
    );
    Ok(())
}
