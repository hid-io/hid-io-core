/* Copyright (C) 2019 by Jacob Alexander
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
use hid_io_core::module::unicode::x11::*;

#[cfg(target_os = "macos")]
use hid_io_core::module::unicode::osx::*;

#[cfg(target_os = "windows")]
use hid_io_core::module::unicode::winapi::*;

use hid_io_core::module::unicode::UnicodeOutput;

pub fn main() {
    #[cfg(target_os = "linux")]
    let mut connection = XConnection::new();
    #[cfg(target_os = "macos")]
    let mut connection = OSXConnection::new();
    #[cfg(target_os = "windows")]
    let mut connection = DisplayConnection::new();

    connection.type_string("💣💩🔥");
}
