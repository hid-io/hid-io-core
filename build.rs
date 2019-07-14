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

// ----- Crates -----

use built;
use capnpc;

// ----- Modules -----

use rustc_version::{version, Version};

// ----- Functions -----

fn main() {
    eprintln!("Compiling for {:?}", std::env::var("CARGO_CFG_TARGET_OS"));

    // Assert if we don't meet the minimum version
    assert!(version().unwrap() >= Version::parse("1.17.0").unwrap());

    // Generate build-time information
    built::write_built_file().expect("Failed to acquire build-time information");

    // Generate Cap'n Proto rust files
    capnpc::CompilerCommand::new()
        .src_prefix("schema")
        .file("schema/common.capnp")
        .file("schema/devicefunction.capnp")
        .file("schema/hidio.capnp")
        .file("schema/hidiowatcher.capnp")
        .file("schema/hostmacro.capnp")
        .file("schema/usbkeyboard.capnp")
        .run()
        .expect("schema compiler command");

    // Link libraries
    match std::env::var("CARGO_CFG_TARGET_OS").unwrap().as_str() {
        "linux" => {
            println!("cargo:rustc-link-lib=X11");
            println!("cargo:rustc-link-lib=Xtst");
        }
        _ => {}
    };
}
