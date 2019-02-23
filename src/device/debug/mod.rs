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

//use std::string;
use std::thread;
//use std::thread::sleep;
use std::time::Duration;

const SLEEP_DURATION: u64 = 1000;

/// debug processing
fn processing() {
    info!("Spawning device/debug spawning thread...");

    // Loop infinitely, the watcher only exits if the daemon is quit
    loop {
        // Sleep so we don't starve the CPU
        thread::sleep(Duration::from_millis(SLEEP_DURATION));
    }
}

/// device debug module initialization
///
/// # Arguments
///
/// # Remarks
///
/// Sets up a processing thread for the debug module.
///
pub fn initialize() {
    info!("Initializing device/debug...");

    // Spawn watcher thread
    thread::Builder::new().name("Debug module".to_string()).spawn(processing).unwrap();
}
