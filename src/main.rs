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

extern crate clap;
extern crate env_logger;
#[macro_use]
extern crate log;



// ----- Modules -----

use clap::App;
use std::{thread, time};

pub mod device;
pub mod module;
pub mod protocol;

pub mod built_info {
    // This file is generated at build time using build.rs
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}



// ----- Functions -----

/// Main entry point
fn main() {
    // Setup logging mechanism
    env_logger::init().unwrap();

    // Process command-line arguments
    // Most of the information is generated from Cargo.toml using built crate (build.rs)
    App::new(format!("{}", built_info::PKG_NAME))
        .version(
            format!("{}{} - {}",
                built_info::PKG_VERSION,
                built_info::GIT_VERSION.map_or_else(
                    || "".to_owned(), |v| format!(" (git {})", v)
                ),
                built_info::PROFILE,
            ).as_str()
        )
        .author(built_info::PKG_AUTHORS)
        .about(format!("\n{}",
                built_info::PKG_DESCRIPTION,
            ).as_str()
        )
        .after_help(format!("{} ({}) -> {} ({})",
                built_info::RUSTC_VERSION,
                built_info::HOST,
                built_info::TARGET,
                built_info::BUILT_TIME_UTC,
            ).as_str()
        )
        .get_matches();

    // Start initialization
    info!("Initializing HID-IO daemon...");

    // Initialize Modules
    module::initialize();

    // Initialize Devices
    device::initialize();

    // XXX (jacob) Is an infinite loop needed here?
    loop {
        thread::sleep(time::Duration::from_millis(2000));
    }

    /*
    debug!("Debug message");
    error!("Error message");
    warn!("Warn message");
    trace!("Trace message");
    */
}

