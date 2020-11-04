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

use crate::api::Endpoint;
use crate::api::HIDAPIInfo;
use crate::common_capnp::NodeType;
use crate::device::*;
use crate::RUNNING;
use lazy_static::lazy_static;
use regex::Regex;
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::{Arc, RwLock};

pub const USAGE_PAGE: u16 = 0xFF1C;
pub const USAGE: u16 = 0x1100;

const USB_FULLSPEED_PACKET_SIZE: usize = 64;
const ENUMERATE_DELAY_MS: u64 = 1000;
const TIMEOUT_MS: i32 = 500;

pub struct HIDAPIDevice {
    device: ::hidapi::HidDevice,
    timeout: i32,
}

impl HIDAPIDevice {
    pub fn new(device: ::hidapi::HidDevice, timeout: i32) -> HIDAPIDevice {
        device.set_blocking_mode(true).unwrap(); // Enable blocking mode, use timeouts to unblock
        HIDAPIDevice { device, timeout }
    }
}

impl std::io::Read for HIDAPIDevice {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self.device.read_timeout(buf, self.timeout) {
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
impl std::io::Write for HIDAPIDevice {
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

impl HidIoTransport for HIDAPIDevice {}

fn device_name(device_info: &::hidapi::DeviceInfo) -> String {
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
fn match_device(device_info: &::hidapi::DeviceInfo) -> bool {
    // NOTE: This requires some patches to hidapi (https://github.com/libusb/hidapi/pull/139)
    // interface number and usage are both queryable. Prefer usage
    device_info.usage_page() == USAGE_PAGE && device_info.usage() == USAGE
}

#[cfg(target_os = "macos")]
fn match_device(device_info: &::hidapi::DeviceInfo) -> bool {
    // interface_number is always -1 but usage is fine
    device_info.usage_page() == USAGE_PAGE && device_info.usage() == USAGE
}

#[cfg(target_os = "windows")]
fn match_device(device_info: &::hidapi::DeviceInfo) -> bool {
    // interface and usage are both queryable. Prefer usage
    device_info.usage_page() == USAGE_PAGE && device_info.usage() == USAGE
}

/// hidapi processing
///
/// This thread periodically refreshes the USB device list to see if a new device needs to be attached
/// The thread also handles reading/writing from connected interfaces
///
/// XXX (HaaTa) hidapi is not thread-safe on all platforms, so don't try to create a thread per device
async fn processing(rt: Arc<tokio::runtime::Runtime>, mailbox: mailbox::Mailbox) {
    info!("Spawning hidapi spawning thread...");

    // Initialize HID interface
    let mut api: ::hidapi::HidApi =
        ::hidapi::HidApi::new().expect("HID API object creation failed");

    // List of allocated device uids
    let uids: Arc<RwLock<HashMap<u64, tokio::task::JoinHandle<()>>>> =
        Arc::new(RwLock::new(HashMap::new()));

    // Loop infinitely, the watcher only exits if the daemon is quit
    // TODO (HaaTa) - There should be a better way using hotplug events (e.g. udev) in a cross
    // platform way
    loop {
        if !RUNNING.load(Ordering::SeqCst) {
            return;
        }

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
            let mut info = HIDAPIInfo::new(device_info);

            // Determine if id can be reused
            // Criteria
            // 1. Must match (even if field isn't valid)
            //    vid, pid, usage page, usage, manufacturer, product, serial, interface
            // 2. Must not currently be in use (generally, use path to differentiate)
            let key = info.key();
            let uid = match mailbox
                .clone()
                .assign_uid(key.clone(), format!("{:#?}", device_info.path()))
            {
                Ok(uid) => uid,
                Err(_) => {
                    // Device has already been registered, or is invalid
                    continue;
                }
            };

            // If serial number is a MAC address, this is a bluetooth device
            lazy_static! {
                static ref RE: Regex =
                    Regex::new(r"([0-9a-fA-F][0-9a-fA-F]:){5}([0-9a-fA-F][0-9a-fA-F])").unwrap();
            }
            let is_ble = RE.is_match(match device_info.serial_number() {
                Some(s) => s,
                _ => "",
            });

            // Basically, we need to copy the path string to deal with lifetime issues
            let device_path = std::ffi::CString::new(device_info.path().to_bytes())
                .expect("hidapi path generation failed");

            // Start thread if uid not it map (i.e. not already processing)
            if !uids.clone().read().unwrap().contains_key(&uid) {
                // Add device
                info!("Connecting to uid:{} {}", uid, device_str);

                // Connect to device
                let hid_device = api.open_path(&device_path);

                // Start thread
                let uids = uids.clone();
                let uids_outer = uids.clone();
                let mailbox = mailbox.clone();
                let handle = rt.clone().spawn_blocking(move || {
                    // Create node
                    let mut node = Endpoint::new(
                        if is_ble {
                            NodeType::BleKeyboard
                        } else {
                            NodeType::UsbKeyboard
                        },
                        uid,
                    );
                    node.set_hidapi_params(info);

                    // Setup device
                    debug!("Attempting to setup {:#?}", node);
                    match hid_device {
                        Ok(device) => {
                            println!("Connected to {}", node);
                            let device = HIDAPIDevice::new(device, TIMEOUT_MS);
                            let mut device = HidIoEndpoint::new(
                                Box::new(device),
                                USB_FULLSPEED_PACKET_SIZE as u32,
                            );

                            // Attempt to synchronize device (sync packet)
                            if let Err(e) = device.send_sync() {
                                // Could not open device (likely removed, or in use)
                                warn!("Failed to sync device - {}", e);
                            } else {
                                // Setup device controller (handles communication and protocol conversion
                                // for the HidIo device)
                                let mut master = HidIoController::new(mailbox.clone(), uid, device);

                                // Add device to node list
                                mailbox.nodes.write().unwrap().push(node);

                                loop {
                                    // Stop processing, daemon trying to quit
                                    if !RUNNING.load(Ordering::SeqCst) {
                                        break;
                                    }

                                    // Process loop for device
                                    let ret = master.process();
                                    if ret.is_err() {
                                        info!("{} disconnected. No longer polling it", uid);
                                        // Remove handle from map
                                        uids.write().unwrap().remove(&uid);

                                        // Remove node from index
                                        {
                                            let mut nodes = mailbox.nodes.write().unwrap();
                                            let index =
                                                nodes.iter().position(|x| x.uid == uid).unwrap();
                                            nodes.remove(index);
                                        }
                                        break;
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            // Could not open device (likely removed, or in use)
                            warn!("Failed to open device:{:?} - {}", device_path, e);
                        }
                    };
                });

                // Add uid to hashmap
                uids_outer.write().unwrap().insert(uid, handle);
            }
        }

        // Sleep so we don't starve the CPU
        // XXX - Rewrite hidapi with rust and include async
        tokio::time::sleep(std::time::Duration::from_millis(ENUMERATE_DELAY_MS)).await;
    }
}

/// hidapi initialization
///
/// Sets up a processing thread for hidapi.
pub async fn initialize(rt: Arc<tokio::runtime::Runtime>, mailbox: mailbox::Mailbox) {
    info!("Initializing device/hidapi...");

    // Spawn watcher thread (tokio)
    rt.clone()
        .spawn_blocking(move || {
            rt.block_on(async {
                let local = tokio::task::LocalSet::new();
                local.run_until(processing(rt.clone(), mailbox)).await;
            });
        })
        .await
        .unwrap();
}
