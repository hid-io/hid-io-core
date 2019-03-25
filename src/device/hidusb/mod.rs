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

use crate::device::*;
use crate::protocol::hidio::*;
use crate::RUNNING;
use hidapi;
use std::sync::atomic::Ordering;

use std::sync::mpsc::channel;
use std::thread;
use std::time::Duration;
use std::time::Instant;

use crate::api::Endpoint;
use crate::common_capnp::NodeType;

// TODO (HaaTa) remove this constants when linux supports better matching
pub const DEV_VID: u16 = 0x308f;
pub const DEV_PID: u16 = 0x0011;
pub const INTERFACE_NUMBER: i32 = 6;

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
                warn!("{:?}", e);
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
                warn!("{:?}", e);
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

fn device_name(device_info: &hidapi::HidDeviceInfo) -> String {
    let mut string = format!(
        "[{:04x}:{:04x}] ",
        device_info.vendor_id, device_info.product_id
    );
    if let Some(m) = &device_info.manufacturer_string {
        string += &m;
    }
    if let Some(p) = &device_info.product_string {
        string += &format!(" {}", p);
    }
    if let Some(s) = &device_info.serial_number {
        string += &format!(" ({})", s);
    }
    string
}

#[cfg(target_os = "linux")]
fn match_device(device_info: &hidapi::HidDeviceInfo) -> bool {
    // usage and usage_page both appear to always be 0, so we can't use them here
    // product_string is also the same for all interfaces so it can't be used
    // fall back to a manual interface number match
    device_info.vendor_id == DEV_VID
        && device_info.product_id == DEV_PID
        && device_info.interface_number == INTERFACE_NUMBER
}

#[cfg(target_os = "macos")]
fn match_device(device_info: &hidapi::HidDeviceInfo) -> bool {
    // interface_number is always -1 but usage is fine
    device_info.usage_page == USAGE_PAGE && device_info.usage == USAGE
}

#[cfg(target_os = "windows")]
fn match_device(device_info: &hidapi::HidDeviceInfo) -> bool {
    // interface and usage are both queryable. Prefer usage
    device_info.usage_page == USAGE_PAGE && device_info.usage == USAGE
}

/// hidusb processing
///
/// This thread periodically refreshes the USB device list to see if a new device needs to be attached
/// The thread also handles reading/writing from connected interfaces
///
/// XXX (HaaTa) hidapi is not thread-safe on all platforms, so don't try to create a thread per device
fn processing(mut mailer: HIDIOMailer) {
    info!("Spawning hidusb spawning thread...");

    // Initialize HID interface
    let mut api = hidapi::HidApi::new().expect("HID API object creation failed");

    let mut devices: Vec<HIDIOController> = vec![];

    let mut last_scan = Instant::now();
    let mut enumerate = true;

    use rand::Rng;
    let mut rng = rand::thread_rng();

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
            info!("Scanning for devices");
            for device_info in api.devices() {
                debug!("{:#x?}", device_info);

                // TODO (HaaTa) Do not use vid, pid + interface number to do match
                // Instead use:
                // 1) bInterfaceClass 0x03 (HID) + bInterfaceSubClass 0x00 (None) + bInterfaceProtocol 0x00 (None)
                // 2) 2 endpoints, EP IN + EP OUT (both Interrupt)
                // 3) iInterface, RawIO API Interface
                if !match_device(device_info) {
                    continue;
                }
                // Add device
                info!("Connecting to {:#?}", device_info);

                let path = device_info.path.clone();

                // TODO: Don't try to connect to a device we are already processing

                // Connect to device
                match api.open_path(&path) {
                    Ok(device) => {
                        println!("Connected to {}", device_name(device_info));
                        let device = HIDUSBDevice::new(device);
                        let mut device =
                            HIDIOEndpoint::new(Box::new(device), USB_FULLSPEED_PACKET_SIZE as u32);

                        let (message_tx, message_rx) = channel::<HIDIOPacketBuffer>();
                        let (response_tx, response_rx) = channel::<HIDIOPacketBuffer>();
                        device.send_sync();

                        let id = rng.gen::<u64>();
                        let master =
                            HIDIOController::new(id.to_string(), device, message_tx, response_rx);
                        devices.push(master);

                        // Add to connected list
                        let info = Endpoint {
                            type_: NodeType::UsbKeyboard,
                            name: device_info
                                .product_string
                                .clone()
                                .unwrap_or_else(|| "[NONE]".to_string()),
                            serial: device_info
                                .serial_number
                                .clone()
                                .unwrap_or_else(|| "".to_string()),
                            id,
                        };
                        let device = HIDIOQueue::new(info, message_rx, response_tx);
                        mailer.register_device(id.to_string(), device);
                    }
                    Err(e) => {
                        // Could not open device (likely removed, or in use)
                        warn!("{}", e);
                        break;
                    }
                };
            }

            if !devices.is_empty() {
                info!("Enumeration finished");
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

            if last_scan.elapsed().as_secs() >= 60 {
                info!("Been a while. Checking for new devices");
                enumerate = true;
                break;
            }

            // Process devices
            devices = devices
                .drain_filter(|dev| {
                    let ret = dev.process();
                    if ret.is_err() {
                        info!("{} disconnected. No loneger polling it", dev.id);
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
pub fn initialize(mailer: HIDIOMailer) {
    info!("Initializing device/hidusb...");

    // Spawn watcher thread
    thread::Builder::new()
        .name("hidusb".to_string())
        .spawn(|| processing(mailer))
        .unwrap();
}
