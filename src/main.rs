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

#[macro_use]
extern crate log;
extern crate env_logger;
#[macro_use]
extern crate serde_derive;

use std::{thread, time};

mod module;
mod device;

// TODO MOVEME





/// Main entry point
fn main() {
    // Setup logging mechanism
    env_logger::init().unwrap();
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

