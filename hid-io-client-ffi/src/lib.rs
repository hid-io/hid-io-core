// Copyright 2022 Jacob Alexander
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

// ----- Crates -----

use c_utf8::CUtf8;
use core::convert::TryFrom;
use core::fmt::Write;
use core::ptr::copy_nonoverlapping;
use cstr_core::c_char;
use cstr_core::CStr;
use hid_io_client::HidioConnection;

// ----- Types -----

// ----- Globals -----

static mut HANDLE: Option<HidioConnection> = None;

// ----- External C Callbacks -----

// ----- External C Interface -----

struct HidioHandle {}

#[repr(C)]
#[derive(PartialEq)]
pub enum HidioStatus {
    /// Command was successful
    Success,
    /// Could not authenticate at the specified auth-level
    ErrorBadAuth,
    /// Could not find hid-io-server connection
    ErrorNoServer,
    /// Not connected to hid-io-server
    ErrorNotConnected,
}

/// Attempt to connect to hid-io-core server
/// True if successful
///
/// This library is not thread safe.
/// Remember to call all functions from the same thread otherwise
/// behaviour is undefined.
/// TODO make sure library works with C
#[no_mangle]
pub extern "C" fn hidio_connect(auth: hid_io_client::AuthType, client_name: String) -> HidioStatus {
    // Prepare hid-io-core connection
    let mut hidio_conn = match hid_io_client::HidioConnection::new() {
        Ok(hidio_conn) => hidio_conn,
        Err(_) => {
            return HidioStatus::ErrorNoServer;
        }
    };

    /*
    let mut rng = rand::thread_rng();

    // Connect and authenticate with hid-io-core
    let (hidio_auth, _hidio_server) = hidio_conn
        .connect(
            hid_io_client::AuthType::Priviledged,
            NodeType::HidioApi,
            "lsnodes".to_string(),
            format!("{:x} - pid:{}", rng.gen::<u64>(), std::process::id()),
            true,
            std::time::Duration::from_millis(1000),
        )
        .await?;
    let hidio_auth = hidio_auth.expect("Could not authenticate to hid-io-core");
    */

    HidioStatus::Success
}

/// Disconnect from hid-io-core server
#[no_mangle]
pub extern "C" fn hidio_disconnect() -> HidioStatus {
    // Check to see if we have a connection handle
    unsafe {
        let handle = match HANDLE.as_mut() {
            Some(handle) => handle,
            None => {
                return HidioStatus::ErrorNotConnected;
            }
        };
    }

    // Verify connection is still valid
    // TODO

    HidioStatus::Success
}
// TODO
// - Connect to hid-io-core (with authentication)
// - Disconnect from hid-io-core
// - Connect to keyboard? (maybe we can just send packets?)
// - Disconnect from keyboard?
// - Check if connected to hid-io-core
// Functions
// - Keyboard info
// - Keyboard layout
// - LED layout
// - LED driver state
// - LED buffer send
// -
