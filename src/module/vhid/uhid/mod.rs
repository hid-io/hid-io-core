/* Copyright (C) 2020 by Jacob Alexander
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

use crate::api::Endpoint;
use crate::api::UhidInfo;
use crate::common_capnp;
use crate::logging::setup_logging_lite;
use crate::mailbox;
use crate::module::vhid;
use crate::protocol::hidio;
use libc::{c_int, c_short, c_ulong, c_void};
use std::io::{Error, ErrorKind};
use std::os::unix::io::AsRawFd;
use uhid_virt;

/// Default uhid device creation
/// Takes care of error scenarios when using ::new() (i.e. panics)
fn create_device(params: uhid_virt::CreateParams) -> uhid_virt::UHIDDevice<std::fs::File> {
    match uhid_virt::UHIDDevice::create(params.clone()) {
        Ok(device) => device,
        Err(msg) => {
            panic!(
                "Could not create {} uhid device (check permissions of /dev/uhid): {}",
                params.name, msg
            );
        }
    }
}

/// Default OutputEvent handler
/// Prints useful debug information when even when the events aren't normally used
fn default_output_event(
    output_event: Result<uhid_virt::OutputEvent, uhid_virt::StreamError>,
    params: uhid_virt::CreateParams,
) -> Result<(), Error> {
    match output_event {
        Ok(event) => match event {
            uhid_virt::OutputEvent::Start { dev_flags } => {
                let mut flags: String = "".to_string();
                for flag in dev_flags {
                    match flag {
                        uhid_virt::DevFlags::FeatureReportsNumbered => {
                            flags += "FeatureReportsNumbered,"
                        }
                        uhid_virt::DevFlags::InputReportsNumbered => {
                            flags += "InputReportsNumbered,"
                        }
                        uhid_virt::DevFlags::OutputReportsNumbered => {
                            flags += "OutputReportsNumbered,"
                        }
                    }
                }
                debug!("Start({}): dev_flags={}", params.name, flags);
                Ok(())
            }
            uhid_virt::OutputEvent::Stop => {
                debug!("Stop({})", params.name);
                Ok(())
            }
            uhid_virt::OutputEvent::Open => {
                debug!("Open({})", params.name);
                Ok(())
            }
            uhid_virt::OutputEvent::Close => {
                debug!("Close({})", params.name);
                Ok(())
            }
            uhid_virt::OutputEvent::Output { data } => {
                debug!("Output({}): {:?}", params.name, data);
                Ok(())
            }
            uhid_virt::OutputEvent::GetReport {
                id,
                report_number,
                report_type,
            } => {
                warn!(
                    "GetReport({}): id={} report_number={} report_type={:?}",
                    params.name, id, report_number, report_type
                );
                Ok(())
            }
            uhid_virt::OutputEvent::SetReport {
                id,
                report_number,
                report_type,
                data,
            } => {
                warn!(
                    "SetReport({}): id={} report_number={} report_type={:?} data={:?}",
                    params.name, id, report_number, report_type, data
                );
                Ok(())
            }
        },
        Err(msg) => {
            match msg {
                // Standard errors (e.g. permission denied)
                uhid_virt::StreamError::Io(err) => Err(err),
                // Unknown errors
                uhid_virt::StreamError::UnknownEventType(code) => Err(Error::new(
                    ErrorKind::Other,
                    format!("Unknown error code: {}", code),
                )),
            }
        }
    }
}

/// uhid NKRO Keyboard
/// To create multiple unique devices, make sure to set uniq to a unique value so to differentiate
/// betweent devices
struct KeyboardNKRO {
    mailbox: mailbox::Mailbox,
    uid: u64,
    endpoint: Endpoint,
    params: uhid_virt::CreateParams,
    device: uhid_virt::UHIDDevice<std::fs::File>,
}

impl KeyboardNKRO {
    pub fn new(
        mailbox: mailbox::Mailbox,
        name: String,
        phys: String,
        uniq: String,
        bus: uhid_virt::Bus,
        vendor: u32,
        product: u32,
        version: u32,
        country: u32,
    ) -> KeyboardNKRO {
        // Setup creation parameters
        let params = uhid_virt::CreateParams {
            name: name.clone(),
            phys,
            uniq,
            bus,
            vendor,
            product,
            version,
            country,
            rd_data: vhid::KEYBOARD_NKRO.to_vec(),
        };

        // Initialize uhid device
        let device = create_device(params.clone());

        // Assign uid to newly created device (need path location for uniqueness)
        let path = "/dev/uhid".to_string();
        let mut uhid_info = UhidInfo::new(params.clone());
        let uid = mailbox.clone().assign_uid(uhid_info.key(), path).unwrap();

        // Setup Endpoint
        let mut endpoint = Endpoint::new(common_capnp::NodeType::HidKeyboard, uid);
        endpoint.set_uhid_params(uhid_info);

        // Register node
        mailbox.clone().register_node(endpoint.clone());

        KeyboardNKRO {
            mailbox,
            uid,
            endpoint,
            params,
            device,
        }
    }

    /// Sends a keyboard HID message
    /// This command does not maintain any state from any previously sent commands
    pub fn send(&mut self, keyboard_hid_codes: Vec<u8>) -> Result<(), Error> {
        // 28 byte message
        let mut data = vec![0; 28];

        // Iterate over hid codes, building the bitmask
        for key in &keyboard_hid_codes {
            match key {
                // 224-231 (1 byte/8 bits) - Modifier Section - Byte 0
                224..=231 => {
                    data[0] |= 1 << (key ^ 0xE0);
                }
                // 4-164 (21 bytes/161 bits + 4 bits + 3 bits) - Keyboard Section - Bytes 1-22
                // 176-221 (6 bytes/46 bits) - Keypad Section
                4..=164 | 176..=221 => {
                    let byte_pos = key / 8; // Determine which byte
                    let bit_mask = 1 << (key - 8 * byte_pos); // Determine which bit
                    data[byte_pos as usize + 1] |= bit_mask; // Offset array by 1 to start at Byte 1
                }
                _ => {}
            };
        }

        // Write message
        match self.device.write(&data) {
            Ok(_) => Ok(()),
            Err(msg) => Err(msg),
        }
    }

    /// Process a single event
    /// This command will block, so make sure to call it in a separate thread
    pub fn process(&mut self) -> Result<(), Error> {
        // Blocks until an event is received
        let output_event = self.device.read();

        // Handle LED events
        if let Ok(event) = &output_event {
            match event {
                uhid_virt::OutputEvent::Output { data } => {
                    // NOTE: data is not processed and is sent as a bitfield
                    // Send message containing LED events
                    self.mailbox.send_command(
                        mailbox::Address::DeviceHid { uid: self.uid },
                        mailbox::Address::All,
                        hidio::HIDIOCommandID::HIDKeyboardLED,
                        data.to_vec(),
                    );
                }
                _ => {}
            }
        }

        // Default event handler
        default_output_event(output_event, self.params.clone())
    }
}

impl Drop for KeyboardNKRO {
    fn drop(&mut self) {
        // Unregister node
        self.mailbox.unregister_node(self.uid);
    }
}

/// uhid 6KRO Keyboard
/// To create multiple unique devices, make sure to set uniq to a unique value so to differentiate
/// betweent devices
struct Keyboard6KRO {
    mailbox: mailbox::Mailbox,
    uid: u64,
    endpoint: Endpoint,
    params: uhid_virt::CreateParams,
    device: uhid_virt::UHIDDevice<std::fs::File>,
}

impl Keyboard6KRO {
    pub fn new(
        mailbox: mailbox::Mailbox,
        name: String,
        phys: String,
        uniq: String,
        bus: uhid_virt::Bus,
        vendor: u32,
        product: u32,
        version: u32,
        country: u32,
    ) -> Keyboard6KRO {
        // Setup creation parameters
        let params = uhid_virt::CreateParams {
            name,
            phys,
            uniq,
            bus,
            vendor,
            product,
            version,
            country,
            rd_data: vhid::KEYBOARD_6KRO.to_vec(),
        };

        // Initialize uhid device
        let device = create_device(params.clone());

        // Assign uid to newly created device (need path location for uniqueness)
        let path = "/dev/uhid".to_string();
        let mut uhid_info = UhidInfo::new(params.clone());
        let uid = mailbox.clone().assign_uid(uhid_info.key(), path).unwrap();

        // Setup Endpoint
        let mut endpoint = Endpoint::new(common_capnp::NodeType::HidKeyboard, uid);
        endpoint.set_uhid_params(uhid_info);

        // Register node
        mailbox.clone().register_node(endpoint.clone());

        Keyboard6KRO {
            mailbox,
            uid,
            endpoint,
            params,
            device,
        }
    }

    /// Sends a keyboard HID message
    /// This command does not maintain any state from any previously sent commands
    pub fn send(&mut self, keyboard_hid_codes: Vec<u8>) -> Result<(), Error> {
        // 8 byte message
        // Byte 0: Modifiers
        // Byte 1: Reserved
        // Byte 2-7: Keys
        let mut data = vec![0; 8];

        // Iterate over hid codes, building message
        let mut key_pos = 2;
        for key in &keyboard_hid_codes {
            match key {
                // 224-231 (1 byte/8 bits) - Modifier Section - Byte 0
                224..=231 => {
                    data[0] |= 1 << (key ^ 0xE0);
                }
                // 4-164, 176-221 (Bytes 2-7)
                4..=164 | 176..=221 => {
                    data[key_pos] = *key;
                    key_pos += 1;
                    if key_pos > 7 {
                        break;
                    }
                }
                _ => {}
            };
        }

        // Write message
        match self.device.write(&data) {
            Ok(_) => Ok(()),
            Err(msg) => Err(msg),
        }
    }

    /// Process a single event
    /// This command will block, so make sure to call it in a separate thread
    pub fn process(&mut self) -> Result<(), Error> {
        // Blocks until an event is received
        let output_event = self.device.read();

        // Handle LED events
        if let Ok(event) = &output_event {
            match event {
                uhid_virt::OutputEvent::Output { data } => {
                    // NOTE: data is not processed and is sent as a bitfield
                    // Send message containing LED events
                    self.mailbox.send_command(
                        mailbox::Address::DeviceHid { uid: self.uid },
                        mailbox::Address::All,
                        hidio::HIDIOCommandID::HIDKeyboardLED,
                        data.to_vec(),
                    );
                }
                _ => {}
            }
        }

        // Default event handler
        default_output_event(output_event, self.params.clone())
    }
}

impl Drop for Keyboard6KRO {
    fn drop(&mut self) {
        // Unregister node
        self.mailbox.unregister_node(self.uid);
    }
}

/*
struct Mouse {
    mailbox: mailbox::Mailbox,
    uid: u64,
    endpoint: Endpoint,
    params: uhid_virt::CreateParams,
    device: uhid_virt::UHIDDevice<std::fs::File>,
}

impl Mouse {
    pub fn new(
        mailbox: mailbox::Mailbox,
        name: String,
        phys: String,
        uniq: String,
        bus: uhid_virt::Bus,
        vendor: u32,
        product: u32,
        version: u32,
        country: u32,
    ) -> Mouse {
        // Setup creation parameters
        let params = uhid_virt::CreateParams {
            name,
            phys,
            uniq,
            bus,
            vendor,
            product,
            version,
            country,
            rd_data: vhid::MOUSE.to_vec(),
        };

        // Initialize uhid device
        let device = create_device(params.clone());

        // Assign uid to newly created device (need path location for uniqueness)
        let path = "/dev/uhid".to_string();
        let mut uhid_info = UhidInfo::new(params.clone());
        let uid = mailbox.clone().assign_uid(uhid_info.key(), path).unwrap();

        // Setup Endpoint
        let mut endpoint = Endpoint::new(common_capnp::NodeType::HidMouse, uid);
        endpoint.set_uhid_params(uhid_info);

        // Register node
        mailbox.clone().register_node(endpoint.clone());

        Mouse { mailbox, uid, endpoint, params, device }
    }
}

impl Drop for Mouse {
    fn drop(&mut self) {
        // Unregister node
        self.mailbox.unregister_node(self.uid);
    }
}

struct Xbox360Controller {
    mailbox: mailbox::Mailbox,
    uid: u64,
    endpoint: Endpoint,
    params: uhid_virt::CreateParams,
    device: uhid_virt::UHIDDevice<std::fs::File>,
}

impl Xbox360Controller {
    pub fn new(
        mailbox: mailbox::Mailbox,
        name: String,
        phys: String,
        uniq: String,
        bus: uhid_virt::Bus,
        vendor: u32,
        product: u32,
        version: u32,
        country: u32,
    ) -> Xbox360Controller {
        // Setup creation parameters
        let params = uhid_virt::CreateParams {
            name,
            phys,
            uniq,
            bus,
            vendor,
            product,
            version,
            country,
            rd_data: vhid::XBOX_360_CONTROLLER.to_vec(),
        };

        // Initialize uhid device
        let device = create_device(params.clone());

        // Assign uid to newly created device (need path location for uniqueness)
        let path = "/dev/uhid".to_string();
        let mut uhid_info = UhidInfo::new(params.clone());
        let uid = mailbox.clone().assign_uid(uhid_info.key(), path).unwrap();

        // Setup Endpoint
        let mut endpoint = Endpoint::new(common_capnp::NodeType::HidJoystick, uid);
        endpoint.set_uhid_params(uhid_info);

        // Register node
        mailbox.clone().register_node(endpoint.clone());

        Xbox360Controller { mailbox, uid, endpoint, params, device }
    }
}

impl Drop for Xbox360Controller {
    fn drop(&mut self) {
        // Unregister node
        self.mailbox.unregister_node(self.uid);
    }
}

struct SysCtrlConsControl {
    mailbox: mailbox::Mailbox,
    uid: u64,
    endpoint: Endpoint,
    params: uhid_virt::CreateParams,
    device: uhid_virt::UHIDDevice<std::fs::File>,
}

impl SysCtrlConsControl {
    pub fn new(
        mailbox: mailbox::Mailbox,
        name: String,
        phys: String,
        uniq: String,
        bus: uhid_virt::Bus,
        vendor: u32,
        product: u32,
        version: u32,
        country: u32,
    ) -> SysCtrlConsControl {
        // Setup creation parameters
        let params = uhid_virt::CreateParams {
            name,
            phys,
            uniq,
            bus,
            vendor,
            product,
            version,
            country,
            rd_data: vhid::SYSCTRL_CONSCTRL.to_vec(),
        };

        // Initialize uhid device
        let device = create_device(params.clone());

        // Assign uid to newly created device (need path location for uniqueness)
        let path = "/dev/uhid".to_string();
        let mut uhid_info = UhidInfo::new(params.clone());
        let uid = mailbox.clone().assign_uid(uhid_info.key(), path).unwrap();

        // Setup Endpoint
        let mut endpoint = Endpoint::new(common_capnp::NodeType::HidKeyboard, uid);
        endpoint.set_uhid_params(uhid_info);

        // Register node
        mailbox.clone().register_node(endpoint.clone());

        SysCtrlConsControl { mailbox, uid, endpoint, params, device }
    }

    /// Process a single event
    /// This command will block, so make sure to call it in a separate thread
    pub fn process(&mut self) -> Result<(), Error> {
        // Blocks until an event is received
        let output_event = self.device.read();

        // Handle LED events
        if let Ok(event) = &output_event {
            match event {
                uhid_virt::OutputEvent::Output { data } => {
                    // NOTE: data is not processed and is sent as a bitfield
                    // Send message containing LED events
                    self.mailbox.send_command(
                        mailbox::Address::DeviceHid { uid: self.uid },
                        mailbox::Address::All,
                        hidio::HIDIOCommandID::HIDKeyboardLED,
                        data.to_vec(),
                    );
                }
                _ => {}
            }
        }

        // Default event handler
        default_output_event(output_event, self.params.clone())
    }
}

impl Drop for SysCtrlConsControl {
    fn drop(&mut self) {
        // Unregister node
        self.mailbox.unregister_node(self.uid);
    }
}
*/

pub async fn initialize(mailbox: mailbox::Mailbox) {
    info!("Initializing vhid/uhid...");
}

// ------- Test Cases -------

#[allow(dead_code)]
#[repr(C)]
struct pollfd {
    fd: c_int,
    events: c_short,
    revents: c_short,
}

#[allow(dead_code)]
#[repr(C)]
struct sigset_t {
    __private: c_void,
}

#[allow(dead_code)]
#[allow(non_camel_case_types)]
type nfds_t = c_ulong;

#[allow(dead_code)]
const POLLIN: c_short = 0x0001;

extern "C" {
    fn ppoll(
        fds: *mut pollfd,
        nfds: nfds_t,
        timeout_ts: *mut libc::timespec,
        sigmask: *const sigset_t,
    ) -> c_int;
}

/// Use parameters to find a uhid device using udev
/// If we don't find the device right away, start to poll
#[allow(dead_code)]
fn udev_find_device(
    vid: u16,
    pid: u16,
    subsystem: String,
    uniq: String,
    timeout: std::time::Duration,
) -> Result<udev::Device, std::io::Error> {
    // First look in the list of devices
    let mut enumerator = udev::Enumerator::new().unwrap();
    enumerator.match_subsystem("input").unwrap();
    enumerator
        .match_attribute("id/vendor", format!("{:04x}", vhid::IC_VID))
        .unwrap();
    enumerator
        .match_attribute("id/product", format!("{:04x}", vhid::IC_PID_KEYBOARD))
        .unwrap();
    enumerator.match_attribute("uniq", uniq.clone()).unwrap();

    // Validate parameters
    for device in enumerator.scan_devices().unwrap() {
        return Ok(device);
    }

    // Couldn't find, setup a watcher

    // Locate hid device with udev
    let mut socket = udev::MonitorBuilder::new()
        .unwrap()
        .match_subsystem(subsystem)
        .unwrap()
        .listen()
        .unwrap();

    // Setup socket polling
    let mut fds = vec![pollfd {
        fd: socket.as_raw_fd(),
        events: POLLIN,
        revents: 0,
    }];

    // Setup ppoll timeout (needed to pump the loop)
    let mut ptimeout = libc::timespec {
        tv_sec: 1,
        tv_nsec: 0,
    };

    // Loop until we find the result
    let start_time = std::time::Instant::now();
    while start_time.elapsed() < timeout {
        // Setup poll
        let result = unsafe {
            ppoll(
                (&mut fds[..]).as_mut_ptr(),
                fds.len() as nfds_t,
                &mut ptimeout,
                std::ptr::null(),
            )
        };

        if result < 0 {
            panic!("Error: {}", std::io::Error::last_os_error());
        }

        // Read message from socket
        let event = match socket.next() {
            Some(evt) => evt,
            None => {
                std::thread::sleep(std::time::Duration::from_millis(10));
                continue;
            }
        };

        // Validate input uhid device
        if event.event_type() == udev::EventType::Add || event.event_type() == udev::EventType::Bind
        {
            // Locate parent
            if let Some(parent) = event.parent() {
                // Match VID:PID
                let found_vid = parent
                    .attribute_value("id/vendor")
                    .unwrap_or(std::ffi::OsStr::new(""))
                    .to_str()
                    .unwrap();
                let found_pid = parent
                    .attribute_value("id/product")
                    .unwrap_or(std::ffi::OsStr::new(""))
                    .to_str()
                    .unwrap();
                let found_uniq = parent
                    .attribute_value("uniq")
                    .unwrap_or(std::ffi::OsStr::new(""))
                    .to_str()
                    .unwrap();
                if found_vid == format!("{:04x}", vid)
                    && found_pid == format!("{:04x}", pid)
                    && found_uniq == uniq
                {
                    return Ok(event.device());
                }
            }
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "Could not locate udev device",
    ))
}

// This test will fail unless your user has permission to read/write to /dev/uhid
#[test]
#[allow_fail]
fn uhid_keyboard_nkro_test() {
    setup_logging_lite();
    let name = "uhid-keyboard-nkro-test".to_string();
    let mailbox = mailbox::Mailbox::new();

    // Generate a unique key (to handle parallel tests)
    let uniq = nanoid::simple();

    // Instantiate hid device
    let mut keyboard = KeyboardNKRO::new(
        mailbox,
        name.clone(),
        "".to_string(),
        uniq.clone(),
        uhid_virt::Bus::USB,
        vhid::IC_VID as u32,
        vhid::IC_PID_KEYBOARD as u32,
        0,
        0,
    );

    // Make sure device is there (will poll for a while just in case uhid/kernel is slow)
    if let Err(msg) = udev_find_device(
        vhid::IC_VID as u16,
        vhid::IC_PID_KEYBOARD as u16,
        "input".to_string(),
        uniq,
        std::time::Duration::new(10, 0),
    ) {
        assert!(false, "Could not find uhid device...");
    }

    // TODO Add evdev support to test keypresses are valid
}

// This test will fail unless your user has permission to read/write to /dev/uhid
#[test]
#[allow_fail]
fn uhid_keyboard_6kro_test() {
    setup_logging_lite();
    let name = "uhid-keyboard-6kro-test".to_string();
    let mailbox = mailbox::Mailbox::new();

    // Generate a unique key (to handle parallel tests)
    let uniq = nanoid::simple();

    // Instantiate hid device
    let mut keyboard = Keyboard6KRO::new(
        mailbox,
        name.clone(),
        "".to_string(),
        uniq.clone(),
        uhid_virt::Bus::USB,
        vhid::IC_VID as u32,
        vhid::IC_PID_KEYBOARD as u32,
        0,
        0,
    );

    // Make sure device is there (will poll for a while just in case uhid/kernel is slow)
    if let Err(msg) = udev_find_device(
        vhid::IC_VID as u16,
        vhid::IC_PID_KEYBOARD as u16,
        "input".to_string(),
        uniq,
        std::time::Duration::new(10, 0),
    ) {
        assert!(false, "Could not find uhid device...");
    }

    // TODO Add evdev support to test keypresses are valid
}
