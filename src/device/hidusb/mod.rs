/* Copyright (C) 2017-2020 by Jacob Alexander
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

use crate::device::*;
use crate::protocol::hidio::*;
use crate::RUNNING;
use lazy_static::lazy_static;
use regex::Regex;
use std::sync::atomic::Ordering;

use std::sync::mpsc::channel;
use std::thread;
use std::time::Duration;
use std::time::Instant;

use crate::api::Endpoint;
use crate::api::HIDAPIInfo;
use crate::common_capnp::NodeType;

pub const USAGE_PAGE: u16 = 0xFF1C;
pub const USAGE: u16 = 0x1100;

const USB_FULLSPEED_PACKET_SIZE: usize = 64;
const ENUMERATE_DELAY: u64 = 1000;
const POLL_DELAY: u64 = 1;

pub struct HIDUSBDevice {
    device: hidapi::HidDevice,
}

impl HIDUSBDevice {
    pub fn new(device: hidapi::HidDevice) -> HIDUSBDevice {
        device.set_blocking_mode(false).unwrap();
        HIDUSBDevice { device }
    }
}

impl std::io::Read for HIDUSBDevice {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self.device.read(buf) {
            Ok(len) => {
                if len > 0 {
                    trace!("Received {} bytes", len);
                    trace!("{:x?}", &buf[0..len]);
                }
                Ok(len)
            }
            Err(e) => {
                warn!("Read - {:?}", e);
                Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("{:?}", e),
                ))
            }
        }
    }
}
impl std::io::Write for HIDUSBDevice {
    fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
        let buf = {
            #[allow(clippy::needless_bool)]
            let prepend = if cfg!(target_os = "linux") || cfg!(target_os = "macos") {
                // If the first byte is a 0 its not tranmitted
                // https://github.com/node-hid/node-hid/issues/187#issuecomment-282863702
                _buf[0] == 0x00
            } else if cfg!(target_os = "windows") {
                // The first byte always seems to be stripped and not tranmitted
                // https://github.com/node-hid/node-hid/issues/187#issuecomment-285688178
                true
            } else {
                // TODO: Test other platforms
                false
            };

            // Add a report id (unused) if needed so our actual first byte
            // of the packet is sent correctly
            if prepend {
                let mut new_buf = vec![0x00];
                new_buf.extend(_buf);
                new_buf
            } else {
                _buf.to_vec()
            }
        };

        match self.device.write(&buf) {
            Ok(len) => {
                trace!("Sent {} bytes", len);
                trace!("{:x?}", &buf[0..len]);
                Ok(len)
            }
            Err(e) => {
                warn!("Write - {:?}", e);
                Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("{:?}", e),
                ))
            }
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl HIDIOTransport for HIDUSBDevice {}

fn device_name(device_info: &hidapi::DeviceInfo) -> String {
    let mut string = format!(
        "[{:04x}:{:04x}-{:x}:{:x}] I:{} ",
        device_info.vendor_id(),
        device_info.product_id(),
        device_info.usage_page(),
        device_info.usage(),
        device_info.interface_number(),
    );
    if let Some(m) = &device_info.manufacturer_string() {
        string += &m;
    }
    if let Some(p) = &device_info.product_string() {
        string += &format!(" {}", p);
    }
    if let Some(s) = &device_info.serial_number() {
        string += &format!(" ({})", s);
    }
    string
}

#[cfg(target_os = "linux")]
fn match_device(device_info: &hidapi::DeviceInfo) -> bool {
    // NOTE: This requires some patches to hidapi (https://github.com/libusb/hidapi/pull/139)
    // interface number and usage are both queryable. Prefer usage
    device_info.usage_page() == USAGE_PAGE && device_info.usage() == USAGE
}

#[cfg(target_os = "macos")]
fn match_device(device_info: &hidapi::DeviceInfo) -> bool {
    // interface_number is always -1 but usage is fine
    device_info.usage_page() == USAGE_PAGE && device_info.usage() == USAGE
}

#[cfg(target_os = "windows")]
fn match_device(device_info: &hidapi::DeviceInfo) -> bool {
    // interface and usage are both queryable. Prefer usage
    device_info.usage_page() == USAGE_PAGE && device_info.usage() == USAGE
}

/// hidusb processing
///
/// This thread periodically refreshes the USB device list to see if a new device needs to be attached
/// The thread also handles reading/writing from connected interfaces
///
/// XXX (HaaTa) hidapi is not thread-safe on all platforms, so don't try to create a thread per device
fn processing(mut mailer: HIDIOMailer, last_uid: Arc<RwLock<u64>>) {
    info!("Spawning hidusb spawning thread...");

    // Initialize HID interface
    let mut api = hidapi::HidApi::new().expect("HID API object creation failed");

    let mut devices: Vec<HIDIOController> = vec![];

    let mut last_scan = Instant::now();
    let mut enumerate = true;

    // Loop infinitely, the watcher only exits if the daemon is quit
    loop {
        while enumerate {
            if !RUNNING.load(Ordering::SeqCst) {
                break;
            }
            last_scan = Instant::now();

            // Refresh devices list
            api.refresh_devices().unwrap();

            // Iterate over found USB interfaces and select usable ones
            debug!("Scanning for devices");
            for device_info in api.device_list() {
                let device_str = format!(
                    "Device: {:#?}\n    {} R:{}",
                    device_info.path(),
                    device_name(device_info),
                    device_info.release_number()
                );
                debug!("{}", device_str);

                // Use usage page and usage for matching HID-IO compatible device
                if !match_device(device_info) {
                    continue;
                }

                // Build set of HID info to make unique comparisons
                let mut info = HIDAPIInfo {
                    path: format!("{:#?}", device_info.path()),
                    vendor_id: device_info.vendor_id(),
                    product_id: device_info.product_id(),
                    serial_number: match device_info.serial_number() {
                        Some(s) => s.to_string(),
                        _ => "<Serial Unset>".to_string(),
                    },
                    release_number: device_info.release_number(),
                    manufacturer_string: match device_info.manufacturer_string() {
                        Some(s) => s.to_string(),
                        _ => "<Manufacturer Unset>".to_string(),
                    },
                    product_string: match device_info.product_string() {
                        Some(s) => s.to_string(),
                        _ => "<Product Unset>".to_string(),
                    },
                    usage_page: device_info.usage_page(),
                    usage: device_info.usage(),
                    interface_number: device_info.interface_number(),
                };

                // Determine if id can be reused
                // Criteria
                // 1. Must match (even if field isn't valid)
                //    vid, pid, usage page, usage, manufacturer, product, serial, interface
                // 2. Must not currently be in use (generally, use path to differentiate)
                let key = info.build_hidapi_key();
                let id = match mailer.get_id(key.clone(), format!("{:#?}", device_info.path())) {
                    Some(0) => {
                        // Device has already been registered
                        continue;
                    }
                    Some(id) => id,
                    None => {
                        // Get last created id and increment
                        (*last_uid.write().unwrap()) += 1;
                        let id = *last_uid.read().unwrap();

                        // Add id to lookup
                        mailer.add_id(key, id);
                        id
                    }
                };

                // Add device
                info!("Connecting to id:{} {}", id, device_str);

                // If serial number is a MAC address, this is a bluetooth device
                lazy_static! {
                    static ref RE: Regex =
                        Regex::new(r"([0-9a-fA-F][0-9a-fA-F]:){5}([0-9a-fA-F][0-9a-fA-F])")
                            .unwrap();
                }
                let is_ble = RE.is_match(match device_info.serial_number() {
                    Some(s) => s,
                    _ => "",
                });

                // Create node
                let mut node = Endpoint::new(
                    if is_ble {
                        NodeType::BleKeyboard
                    } else {
                        NodeType::UsbKeyboard
                    },
                    id,
                );
                node.set_hidapi_params(info);

                // Connect to device
                debug!("Attempt to open {:#?}", node);
                match api.open_path(device_info.path()) {
                    Ok(device) => {
                        println!("Connected to {}", node);
                        let device = HIDUSBDevice::new(device);
                        let mut device =
                            HIDIOEndpoint::new(Box::new(device), USB_FULLSPEED_PACKET_SIZE as u32);

                        let (message_tx, message_rx) = channel::<HIDIOPacketBuffer>();
                        let (response_tx, response_rx) = channel::<HIDIOPacketBuffer>();
                        device.send_sync();

                        let master =
                            HIDIOController::new(id.to_string(), device, message_tx, response_rx);
                        devices.push(master);

                        // Add to connected list
                        let device = HIDIOQueue::new(node, message_rx, response_tx);
                        mailer.register_device(id.to_string(), device);
                    }
                    Err(e) => {
                        // Could not open device (likely removed, or in use)
                        warn!("Processing - {}", e);
                        break;
                    }
                };
            }

            if !devices.is_empty() {
                debug!("Enumeration finished");
                enumerate = false;
                break;
            }

            // Sleep so we don't starve the CPU
            // TODO (HaaTa) - There should be a better way to watch the ports, but still be responsive
            thread::sleep(Duration::from_millis(ENUMERATE_DELAY));
        }

        loop {
            if !RUNNING.load(Ordering::SeqCst) {
                break;
            }

            if devices.is_empty() {
                info!("No connected devices. Forcing scan");
                enumerate = true;
                break;
            }

            // TODO (HaaTa): Make command-line argument/config option
            if last_scan.elapsed().as_secs() >= 1 {
                debug!("Been a while. Checking for new devices");
                enumerate = true;
                break;
            }

            // Process devices
            devices = devices
                .drain_filter(|dev| {
                    let ret = dev.process();
                    if ret.is_err() {
                        info!("{} disconnected. No longer polling it", dev.id);
                        mailer.unregister_device(&dev.id);
                    }
                    ret.is_ok()
                })
                .collect::<Vec<_>>();

            mailer.process();

            // TODO (HaaTa) - If there was any IO, on any of the devices, do not sleep, only sleep when all devices are idle
            thread::sleep(Duration::from_millis(POLL_DELAY));
        }
    }
}

/// hidusb initialization
///
/// Sets up a processing thread for hidusb.
pub fn initialize(mailer: HIDIOMailer, last_uid: Arc<RwLock<u64>>) {
    info!("Initializing device/hidusb...");

    // Spawn watcher thread
    thread::Builder::new()
        .name("hidusb".to_string())
        .spawn(|| processing(mailer, last_uid))
        .unwrap();
}
