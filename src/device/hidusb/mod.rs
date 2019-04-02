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

use crate::device::*;
use crate::protocol::hidio::*;
use crate::RUNNING;
use hidapi;
use libusb;
use std::sync::atomic::Ordering;

use std::sync::mpsc::channel;
use std::thread;
use std::time::Duration;
use std::time::Instant;

use crate::api::Endpoint;
use crate::common_capnp::NodeType;

pub const HIDIO_USAGE_PAGE: u16 = 0xFF1C;
pub const HIDIO_USAGE: u16 = 0x1100;
pub const KEYBOARD_USAGE_PAGE: u16 = 0x0001;
pub const KEYBOARD_USAGE: u16 = 0x0006;
pub const MOUSE_USAGE_PAGE: u16 = 0x0001;
pub const MOUSE_USAGE: u16 = 0x0002;

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
    // XXX (HaaTa) usage and usage_page requires patched version of hidapi using hidraw
    device_info.usage_page == HIDIO_USAGE_PAGE && device_info.usage == HIDIO_USAGE
}

#[cfg(target_os = "macos")]
fn match_device(device_info: &hidapi::HidDeviceInfo) -> bool {
    // interface_number is always -1 but usage is fine
    device_info.usage_page == HIDIO_USAGE_PAGE && device_info.usage == HIDIO_USAGE
}

#[cfg(target_os = "windows")]
fn match_device(device_info: &hidapi::HidDeviceInfo) -> bool {
    // interface and usage are both queryable. Prefer usage
    device_info.usage_page == HIDIO_USAGE_PAGE && device_info.usage == HIDIO_USAGE
}

fn match_keyboard(device_info: &hidapi::HidDeviceInfo) -> bool {
    device_info.usage_page == KEYBOARD_USAGE_PAGE && device_info.usage == KEYBOARD_USAGE
}

fn match_usb(device_info: &hidapi::HidDeviceInfo, device_desc: &libusb::DeviceDescriptor) -> bool {
    device_info.vendor_id == device_desc.vendor_id() && device_info.product_id == device_desc.product_id()
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

    let usb_context = libusb::Context::new().unwrap();

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

                // Use only parts of HID descriptor for matching:
                // USAGE_PAGE
                // USAGE
                let mut proceed = false;
                if match_device(device_info) {
                    println!("HIDIO! {}", device_name(device_info));
                    proceed = true;
                } else if match_keyboard(device_info) {
                    println!("Keyboard! {}", device_name(device_info));
                } else {
                    continue;
                }

                // Gather USB Descriptor information
                // TODO Filter better than using iter and enumerate
                for device in usb_context.devices().unwrap().iter() {
                    let device_desc = device.device_descriptor().unwrap();
                    if match_usb(device_info, &device_desc) {
                    println!("  Bus {:03} Device {:03} ID {:04x}:{:04x}",
                        device.bus_number(),
                        device.address(),
                        device_desc.vendor_id(),
                        device_desc.product_id());
                        let config = device.active_config_descriptor().unwrap();
                        for (iface_num, iface_enum) in config.interfaces().enumerate() {
                            for (_, interface) in iface_enum.descriptors().enumerate() {

                                if device_info.interface_number as usize == iface_num {
                                    println!("  Iface Num: {} Class: {} Sub-Class: {} Protocol: {}",
                                        iface_num,
                                        interface.class_code(),
                                        interface.sub_class_code(),
                                        interface.protocol_code());
                                    for (_, endpoint) in interface.endpoint_descriptors().enumerate() {
                                        println!("  Endpoint: {} Addr: {} Dir: {:?} Type: {:?} Max Packet: {} Interval: {}",
                                            endpoint.number(),
                                            endpoint.address(),
                                            endpoint.direction(),
                                            endpoint.transfer_type(),
                                            endpoint.max_packet_size(),
                                            endpoint.interval());
                                    }
                                }
                            }
                        }
                    }
                }

                // Only continue if completely matched and supported
                if !proceed {
                    continue;
                }

                // Add device
                info!("Connecting to {:#?}", device_info);

                let path = device_info.path.clone();

                // TODO: Don't try to connect to a device we are already processing

                // Connect to device
                match api.open_path(&path) {
                    Ok(device) => {
                        println!("HID-IO -> {}", device_name(device_info));
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
                        // TODO (HaaTa): Should distinguish between USB and HID devices
                        //               as USB HID devices have more information that can be
                        //               queried
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
