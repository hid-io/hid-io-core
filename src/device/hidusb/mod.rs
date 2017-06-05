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

extern crate hidapi;

use std::string;
use std::thread;
use std::thread::sleep;
use std::time::Duration;


// TODO (HaaTa) remove this constants when hidapi supports better matching
const DEV_VID: u16 = 0x1c11;
const DEV_PID: u16 = 0xb04d;
const INTERFACE_NUMBER: i32 = 5;

const PACKET_SIZE: usize = 64; // TODO Autodetect
const SLEEP_DURATION: u64 = 1000;


/// HIDUSBDevice Struct
///
/// Contains HIDUSB device thread information
/// Required to communicate with device thread
struct HIDUSBDevice<'a> {
    device     : hidapi::HidDevice<'a>,
    deviceinfo : hidapi::HidDeviceInfo,
}


impl<'a> HIDUSBDevice<'a> {
    // hidusb device processing
    fn process(&self) -> hidapi::HidResult<usize> {
        println!("yay");

            // Send dummy command (REMOVEME)
            let res = self.device.write("Test".as_bytes());
            match res {
                Ok(result) => {
                    println!("Ok");
                    println!("Retval: {}", result);
                },
                Err(e) => {
                    warn!("Warning! {}", e);
                    return res;
                },
            }
            //println!("{}", self.device.check_error().unwrap());
            return res;

            /*
            let res = device_list[index].0.get_indexed_string(1).unwrap();
            let res = device_list[index].0.get_indexed_string(10).unwrap();
            println!("Retval: {}", res);
            */
    }

}


/// hidusb processing
///
/// This thread periodically refreshes the USB device list to see if a new device needs to be attached
/// The thread also handles reading/writing from connected interfaces
///
/// XXX (HaaTa) hidapi is not thread-safe on all platforms, so don't try to create a thread per device
fn processing() {
    info!("Spawning hidusb spawning thread...");

    // Initialize HID interface
    let mut api = hidapi::HidApi::new().expect("HID API object creation failed");

    // Connected devices/threads
    let mut device_list: Vec<HIDUSBDevice> = Vec::new();


    /// Loop infinitely, the watcher only exits if the daemon is quit
    loop {
        // Refresh devices list
        // TODO
        //api.refresh_devices();

        // Connected/validated device info
        let mut device_info_list: Vec<hidapi::HidDeviceInfo> = Vec::new();


        // Iterate over found USB interfaces and select usable ones
        for device_info in api.devices() {
            debug!("{:#?}", device_info);

            // TODO (HaaTa) Do not use vid, pid + interface number to do match
            // Instead use:
            // 1) bInterfaceClass 0x03 (HID) + bInterfaceSubClass 0x00 (None) + bInterfaceProtocol 0x00 (None)
            // 2) 2 endpoints, EP IN + EP OUT (both Interrupt)
            // 3) iInterface, RawIO API Interface
            if !( device_info.vendor_id == DEV_VID && device_info.product_id == DEV_PID && device_info.interface_number == INTERFACE_NUMBER ) {
                continue;
            }

            // Interface found, add to list
            device_info_list.push(device_info)
        }


        // Connect to each device where a connection does not yet exist
        'outer: for device_info in device_info_list {
            // Make sure we haven't already connected
            for device in &device_list {
                if device_info.path == device.deviceinfo.path {
                    continue 'outer;
                }
            }

            info!("Connecting to {:#?}", device_info);

            // Add to connected list
            let path = device_info.path.clone();

            // Connect to device
            let device = api.open_path(&path).expect("Failed to open device");

            // Add device to processing list
            device_list.push(HIDUSBDevice {deviceinfo: device_info, device: device});
        }

        // Iterate over devices doing writes and reads
        let mut remove_list: Vec<usize> = Vec::new();
        for (index, device) in device_list.iter().enumerate() {
            // Process device
            match device.process() {
                Ok(result) => {},
                Err(e) => {
                    info!("Removing {:#?}", device.deviceinfo);
                    remove_list.push(index);
                },
            }
        }

        // Remove problematic devices, will be re-added on the next loop if available
        for index in remove_list {
            device_list.remove(index);
        }


        // Sleep so we don't starve the CPU
        thread::sleep(Duration::from_millis(SLEEP_DURATION));
    }
}


/// hidusb initialization
///
/// Sets up a processing thread for hidusb.
pub fn initialize() {
    info!("Initializing hidusb...");

    // Spawn watcher thread
    thread::spawn(processing);
}

