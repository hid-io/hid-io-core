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

extern crate built;
extern crate capnpc;
extern crate rustc_version;



// ----- Modules -----

use rustc_version::{version, Version};



// ----- Functions -----

fn main() {
    // Assert if we don't meet the minimum version
    assert!(version().unwrap() >= Version::parse("1.17.0").unwrap());

    // Generate build-time information
    built::write_built_file().expect("Failed to acquire build-time information");

    // Generate Cap'n Proto rust files
    /*
    capnpc::CompilerCommand::new()
        .src_prefix("schema")
        .file("schema/test.capnp")
        .run()
        .expect("schema compiler command");
    */
}

