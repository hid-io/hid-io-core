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

pub mod hidusb;
pub mod debug;

/// Module initialization
///
/// # Remarks
///
/// Sets up at least one thread per Device.
/// Each device is repsonsible for accepting and responding to packet requests.
/// It is also possible to send requests asynchronously back to any Modules.
/// Each device may have it's own RPC API.
pub fn initialize() {
    info!("Initializing devices...");

    // Initialize device watcher threads
    hidusb::initialize();
    debug::initialize();
}

