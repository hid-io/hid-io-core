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
use crate::api::EvdevInfo;
use crate::common_capnp::NodeType;
use crate::device::*;
use crate::RUNNING;
use evdev_rs;
use lazy_static::lazy_static;
use regex::Regex;
use std::sync::atomic::Ordering;
use std::time::Instant;

/// Device state container for evdev devices
pub struct EvdevDevice {
    fd_path: String,
}

impl EvdevDevice {
    pub fn new(fd_path: String) -> EvdevDevice {
        EvdevDevice { fd_path }
    }

    // Process evdev events
    pub async fn process(&mut self) -> Result<(), std::io::Error> {
        let fd_path = self.fd_path.clone();

        tokio::task::spawn_blocking(move || {
            // Initialize new evdev handle
            let mut device = evdev_rs::Device::new().unwrap();

            // Apply file descriptor to evdev handle
            let file = std::fs::File::open(fd_path).unwrap();
            device.set_fd(file).unwrap();

            // TODO Read all the necessary device fields for Endpoint (or perhaps inside new?)

            // Take all event information (block events from other processes)
            device.grab(evdev_rs::GrabMode::Grab).unwrap();

            let mut event: std::io::Result<(evdev_rs::ReadStatus, evdev_rs::InputEvent)>;
            // Continuously scan for new events
            // This loop will block at next_event()
            loop {
                event =
                    device.next_event(evdev_rs::ReadFlag::NORMAL | evdev_rs::ReadFlag::BLOCKING);
                if event.is_ok() {
                    let mut result = event.ok().unwrap();
                    match result.0 {
                        evdev_rs::ReadStatus::Sync => {
                            // Dropped packet (this shouldn't happen)
                            // We should warn about it though
                            warn!("Dropped evdev event! - Attempting to resync...");
                            while result.0 == evdev_rs::ReadStatus::Sync {
                                // TODO show dropped event
                                //print_sync_dropped_event(&result.1);
                                event = device.next_event(evdev_rs::ReadFlag::SYNC);
                                if event.is_ok() {
                                    result = event.ok().unwrap();
                                } else {
                                    break;
                                }
                            }
                            warn!("Resyncing successful.");
                        }
                        evdev_rs::ReadStatus::Success => {
                            // TODO send event message through mailbox
                            //print_event(&result.1),
                        }
                    }
                } else {
                    // Disconnection event, shutdown processing loop
                    // This object should be deallocated as well
                    let err = event.err().unwrap();
                    match err.raw_os_error() {
                        Some(libc::EAGAIN) => continue,
                        _ => {
                            info!("Disconnection event {}", device_name(device));
                            break;
                        }
                    }
                }
            }
        })
        .await?;
        Ok(())
    }
}

/// Build a unique device name string
fn device_name(device: evdev_rs::Device) -> String {
    let mut string = format!(
        "[{:04x}:{:04x}-{:?}] {} {} {}",
        device.vendor_id(),
        device.product_id(),
        evdev_rs::enums::int_to_bus_type(device.bustype() as u32),
        device.name().unwrap_or(""),
        device.phys().unwrap_or(""),
        device.uniq().unwrap_or(""),
    );
    string
}

/// evdev processing
///
/// TODO
/// udev to wait on new evdev devices
/// udev to scan for already attached devices
/// Allocate uid per unique device
/// Have list of evdev devices to query
/// Handle removal and re-insertion with same uid
/// Use async to wait for evdev events (block on next event, using spawn_blocking)
/// Send mailbox message with necessary info (API will handle re-routing message)

/// hidapi processing
///
/// This thread periodically refreshes the USB device list to see if a new device needs to be attached
/// The thread also handles reading/writing from connected interfaces
///
/// XXX (HaaTa) hidapi is not thread-safe on all platforms, so don't try to create a thread per device
/*
async fn processing(mut mailbox: mailbox::Mailbox) {
    info!("Spawning hidapi spawning thread...");

    // Initialize HID interface
    let mut api = ::hidapi::HidApi::new().expect("HID API object creation failed");

    let mut devices: Vec<HIDIOController> = vec![];

    let mut last_scan = Instant::now();
    let mut enumerate = true;

    // Loop infinitely, the watcher only exits if the daemon is quit
    loop {
        while enumerate {
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
                let key = info.build_hidapi_key();
                let uid = match mailbox.get_uid(key.clone(), format!("{:#?}", device_info.path())) {
                    Some(0) => {
                        // Device has already been registered
                        continue;
                    }
                    Some(uid) => uid,
                    None => {
                        // Get last created id and increment
                        (*mailbox.last_uid.write().unwrap()) += 1;
                        let uid = *mailbox.last_uid.read().unwrap();

                        // Add id to lookup
                        mailbox.add_uid(key, uid);
                        uid
                    }
                };

                // Check to see if already connected
                if devices.iter().any(|dev| dev.uid == uid) {
                    continue;
                }

                // Add device
                info!("Connecting to uid:{} {}", uid, device_str);

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
                    uid,
                );
                node.set_hidapi_params(info);

                // Connect to device
                debug!("Attempt to open {:#?}", node);
                match api.open_path(device_info.path()) {
                    Ok(device) => {
                        println!("Connected to {}", node);
                        let device = HIDAPIDevice::new(device);
                        let mut device =
                            HIDIOEndpoint::new(Box::new(device), USB_FULLSPEED_PACKET_SIZE as u32);

                        if let Err(e) = device.send_sync() {
                            // Could not open device (likely removed, or in use)
                            warn!("Processing - {}", e);
                            continue;
                        }

                        // Setup device controller (handles communication and protocol conversion
                        // for the HIDIO device)
                        let master = HIDIOController::new(mailbox.clone(), uid, device);
                        devices.push(master);

                        // Add device to node list
                        mailbox.nodes.write().unwrap().push(node);
                    }
                    Err(e) => {
                        // Could not open device (likely removed, or in use)
                        warn!("Processing - {}", e);
                        continue;
                    }
                };
            }

            // Update scan time
            last_scan = Instant::now();

            if !devices.is_empty() {
                debug!("Enumeration finished");
                enumerate = false;
            } else {
                // Sleep so we don't starve the CPU
                // TODO (HaaTa) - There should be a better way to watch the ports, but still be responsive
                // XXX - Rewrite hidapi with rust and include async
                tokio::time::delay_for(std::time::Duration::from_millis(ENUMERATE_DELAY)).await;
            }
        }

        loop {
            if !RUNNING.load(Ordering::SeqCst) {
                return;
            }

            if devices.is_empty() {
                info!("No connected devices. Forcing scan");
                enumerate = true;
                break;
            }

            // TODO (HaaTa): Make command-line argument/config option
            if last_scan.elapsed().as_millis() >= 1000 {
                enumerate = true;
                break;
            }

            // Process devices
            let mut removed_devices = vec![];
            let mut io_events: usize = 0;
            devices = devices
                .drain_filter(|dev| {
                    // Check if disconnected
                    let ret = dev.process();
                    let result = ret.is_ok();
                    if ret.is_err() {
                        removed_devices.push(dev.uid);
                        info!("{} disconnected. No longer polling it", dev.uid);
                    } else {
                        // Record io events (used to schedule sleeps)
                        io_events += ret.ok().unwrap();
                    }
                    result
                })
                .collect::<Vec<_>>();

            // Modify nodes list to remove any uids that were disconnected
            // uids are unique across both api and devices, so this is always safe to do
            if !removed_devices.is_empty() {
                let new_nodes = mailbox
                    .nodes
                    .read()
                    .unwrap()
                    .clone()
                    .drain_filter(|node| !removed_devices.contains(&node.uid()))
                    .collect::<Vec<_>>();
                *mailbox.nodes.write().unwrap() = new_nodes;
            }

            // If there was any IO, on any of the devices, do not sleep, only sleep when all devices are idle
            if io_events == 0 {
                tokio::time::delay_for(std::time::Duration::from_millis(POLL_DELAY)).await;
            }
        }
    }
}
*/

/// evdev initialization
///
/// Sets up processing threads for udev and evdev.
pub async fn initialize(mailbox: mailbox::Mailbox) {
    info!("Initializing device/evdev...");

    // Spawn watcher thread (tokio)
    /*
    let local = tokio::task::LocalSet::new();
    local.run_until(processing(mailbox)).await;
    */
}

#[test]
fn uhid_evdev_keyboard_test() {
    // Create uhid keyboard interface
    //println!("YAY");
}
