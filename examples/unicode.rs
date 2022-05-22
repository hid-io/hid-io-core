#![cfg(feature = "displayserver")]
/* Copyright (C) 2019-2022 by Jacob Alexander
 * Copyright (C) 2019 by Rowan Decker
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

#[cfg(target_os = "linux")]
use hid_io_core::module::displayserver::x11::*;

#[cfg(target_os = "linux")]
use hid_io_core::module::displayserver::wayland::*;

#[cfg(target_os = "macos")]
use hid_io_core::module::displayserver::quartz::*;

#[cfg(target_os = "windows")]
use hid_io_core::module::displayserver::winapi::*;

use hid_io_core::module::displayserver::DisplayOutput;

#[cfg(target_os = "linux")]
fn get_display() -> Box<dyn DisplayOutput> {
    // First attempt to connect to Wayland
    let wayland = WaylandConnection::new();
    if let Ok(wayland) = wayland {
        Box::new(wayland)

    // Then fallback to X11
    } else {
        Box::new(XConnection::new())
    }
}

pub fn main() {
    hid_io_core::logging::setup_logging_lite().unwrap();

    #[cfg(target_os = "linux")]
    let mut connection = get_display();
    #[cfg(target_os = "macos")]
    let mut connection = QuartzConnection::new();
    #[cfg(target_os = "windows")]
    let mut connection = DisplayConnection::new();

    log::info!("START");
    connection.type_string("💣💩🔥").unwrap(); // Test unicode
    connection.type_string("abc💣💩🔥🔥").unwrap(); // Test quickly repeated unicode
    connection.type_string("\n").unwrap(); // Test enter
    connection.type_string("carg\t --help\n").unwrap(); // Test tab and command

    connection.set_held("def").unwrap();
    connection.set_held("gアi").unwrap();
    std::thread::sleep(std::time::Duration::from_millis(1000)); // Test hold
    connection.set_held("gア").unwrap();
    std::thread::sleep(std::time::Duration::from_millis(1000)); // Test partial release
    connection.set_held("").unwrap();
    log::info!("END");
}
