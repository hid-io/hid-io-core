/* Copyright (C) 2017-2022 by Jacob Alexander
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

// ----- Modules -----

#[cfg(feature = "api")]
mod capnp;

// ----- Crates -----

#[cfg(feature = "api")]
pub use crate::common_capnp;

#[cfg(all(feature = "dev-capture", target_os = "linux"))]
use evdev_rs::DeviceWrapper;

use crate::mailbox;
use hid_io_protocol::HidIoCommandId;
use std::time::Instant;

// ----- Functions -----

/// Authorization level for a remote node
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AuthLevel {
    /// Allows connecting and listing devices
    Basic,
    /// Allows sending commands to a device
    Secure,
    /// Allows inspecting all incoming packets
    Debug,
}

/// Uhid Information
/// This is only used on Linux
#[derive(Debug, Clone, Default)]
pub struct UhidInfo {
    // These fields are from uhid_virt::CreateParams
    pub name: String,
    pub phys: String,
    pub uniq: String,
    pub bus: u16,
    pub vendor: u32,
    pub product: u32,
    pub version: u32,
    pub country: u32,
}

impl UhidInfo {
    /// Generate a unique string based off of evdev information (excluding path/physical location)
    pub fn key(&mut self) -> String {
        format!(
            "vendor:{:04x} product:{:04x} name:{} phys:{} uniq:{} bus:{} version:{} country:{}",
            self.vendor,
            self.product,
            self.name,
            self.phys,
            self.uniq,
            self.bus,
            self.version,
            self.country,
        )
    }

    #[cfg(all(feature = "vhid", target_os = "linux"))]
    pub fn new(params: uhid_virt::CreateParams) -> UhidInfo {
        UhidInfo {
            name: params.name,
            phys: params.phys,
            uniq: params.uniq,
            bus: params.bus as u16,
            vendor: params.vendor,
            product: params.product,
            version: params.version,
            country: params.country,
        }
    }
}

/// Evdev Information
/// This is only used on Linux
#[derive(Debug, Clone, Default)]
pub struct EvdevInfo {
    // These fields are from evdev_rs::Device
    pub name: String,
    pub phys: String,
    pub uniq: String,
    pub product_id: u16,
    pub vendor_id: u16,
    pub bustype: u16,
    pub version: u16,
    pub driver_version: i32,
}

impl EvdevInfo {
    /// Generate a unique string based off of evdev information (excluding path/physical location)
    pub fn key(&mut self) -> String {
        format!(
            "vid:{:04x} pid:{:04x} name:{} phys:{} uniq:{} bus:{} version:{} driver_version:{}",
            self.vendor_id,
            self.product_id,
            self.name,
            self.phys,
            self.uniq,
            self.bustype,
            self.version,
            self.driver_version,
        )
    }

    #[cfg(all(feature = "dev-capture", target_os = "linux"))]
    pub fn new(device: evdev_rs::Device) -> EvdevInfo {
        EvdevInfo {
            name: device.name().unwrap_or("").to_string(),
            phys: device.phys().unwrap_or("").to_string(),
            uniq: device.uniq().unwrap_or("").to_string(),
            product_id: device.product_id(),
            vendor_id: device.vendor_id(),
            bustype: device.bustype(),
            version: device.version(),
            driver_version: device.driver_version(),
        }
    }
}

/// HidApi Information
#[derive(Debug, Clone, Default)]
pub struct HidApiInfo {
    pub path: String,
    pub vendor_id: u16,
    pub product_id: u16,
    pub serial_number: String,
    pub release_number: u16,
    pub manufacturer_string: String,
    pub product_string: String,
    pub usage_page: u16,
    pub usage: u16,
    pub interface_number: i32,
}

impl HidApiInfo {
    /// Generate a unique string based off of hidapi information (excluding path/physical location)
    pub fn key(&mut self) -> String {
        format!(
            "vid:{:04x} pid:{:04x} serial:{} manufacturer:{} product:{} usage_page:{:x} usage:{:x} interface:{}",
            self.vendor_id,
            self.product_id,
            self.serial_number,
            self.manufacturer_string,
            self.product_string,
            self.usage_page,
            self.usage,
            self.interface_number,
        )
    }

    #[cfg(feature = "hidapi-devices")]
    pub fn new(device_info: &hidapi::DeviceInfo) -> HidApiInfo {
        HidApiInfo {
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
        }
    }
}

/// Dummy enum when api is not being compiled in
#[cfg(not(feature = "api"))]
pub mod common_capnp {
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub enum NodeType {
        BleKeyboard,
        HidJoystick,
        HidKeyboard,
        HidMouse,
        HidioDaemon,
        UsbKeyboard,
    }
}

/// Information about a connected node
#[derive(Debug, Clone)]
pub struct Endpoint {
    type_: common_capnp::NodeType,
    name: String,   // Used for hidio (e.g. hidioDaemon, hidioApi) types
    serial: String, // Used for hidio (e.g. hidioDaemon, hidioApi) types
    pub uid: u64,
    created: Instant,
    hidapi: HidApiInfo,
    evdev: EvdevInfo,
    uhid: UhidInfo,
}

impl std::fmt::Display for Endpoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(
            format!(
                "id:{} {} {}",
                self.uid,
                match self.type_ {
                    common_capnp::NodeType::BleKeyboard => format!(
                        "BLE [{:04x}:{:04x}-{:x}:{:x}] {}",
                        self.hidapi.vendor_id,
                        self.hidapi.product_id,
                        self.hidapi.usage_page,
                        self.hidapi.usage,
                        self.hidapi.product_string,
                    ),
                    common_capnp::NodeType::UsbKeyboard => format!(
                        "USB [{:04x}:{:04x}-{:x}:{:x}] [{}] {}",
                        self.hidapi.vendor_id,
                        self.hidapi.product_id,
                        self.hidapi.usage_page,
                        self.hidapi.usage,
                        self.hidapi.manufacturer_string,
                        self.hidapi.product_string,
                    ),
                    // TODO Display Hid devices, but handle in a cross-platform way
                    _ => self.name.clone(),
                },
                match self.type_ {
                    common_capnp::NodeType::BleKeyboard | common_capnp::NodeType::UsbKeyboard =>
                        self.hidapi.serial_number.clone(),
                    _ => self.serial.clone(),
                },
            )
            .as_str(),
        )
    }
}

impl Endpoint {
    pub fn new(type_: common_capnp::NodeType, uid: u64) -> Endpoint {
        Endpoint {
            type_,
            name: "".to_string(),
            serial: "".to_string(),
            uid,
            created: Instant::now(),
            hidapi: HidApiInfo {
                ..Default::default()
            },
            evdev: EvdevInfo {
                ..Default::default()
            },
            uhid: UhidInfo {
                ..Default::default()
            },
        }
    }

    pub fn set_daemonnode_params(&mut self) {
        self.name = "HID-IO Core Daemon Node".to_string();
        self.serial = format!("pid:{}", std::process::id());
    }

    pub fn set_evdev_params(&mut self, info: EvdevInfo) {
        self.evdev = info;
        self.name = self.name();
        self.serial = self.serial();
    }

    pub fn set_hidio_params(&mut self, name: String, serial: String) {
        self.name = name;
        self.serial = serial;
    }

    pub fn set_hidapi_params(&mut self, info: HidApiInfo) {
        self.hidapi = info;
        self.name = self.name();
        self.serial = self.serial();
    }

    pub fn set_uhid_params(&mut self, info: UhidInfo) {
        self.uhid = info;
        self.name = self.name();
        self.serial = self.serial();
    }

    pub fn set_hidapi_path(&mut self, path: String) {
        self.hidapi.path = path;
    }

    pub fn type_(&mut self) -> common_capnp::NodeType {
        self.type_
    }

    pub fn name(&mut self) -> String {
        match self.type_ {
            common_capnp::NodeType::BleKeyboard => format!(
                "[{:04x}:{:04x}-{:x}:{:x}] {}",
                self.hidapi.vendor_id,
                self.hidapi.product_id,
                self.hidapi.usage_page,
                self.hidapi.usage,
                self.hidapi.product_string,
            ),
            common_capnp::NodeType::UsbKeyboard => format!(
                "[{:04x}:{:04x}-{:x}:{:x}] [{}] {}",
                self.hidapi.vendor_id,
                self.hidapi.product_id,
                self.hidapi.usage_page,
                self.hidapi.usage,
                self.hidapi.manufacturer_string,
                self.hidapi.product_string,
            ),
            _ => self.name.clone(),
        }
    }

    /// Used to generate a unique key that will point to this device
    /// Empty fields are still used (in the case of bluetooth and the interface field on Windows
    /// sometimes)
    /// Does not include path, as the path may not uniquely identify device port or device
    /// Does not include release number as this may be incrementing
    pub fn key(&mut self) -> String {
        match self.type_ {
            common_capnp::NodeType::BleKeyboard | common_capnp::NodeType::UsbKeyboard => {
                self.hidapi.key()
            }
            _ => format!("name:{} serial:{}", self.name, self.serial,),
        }
    }

    pub fn serial(&mut self) -> String {
        match self.type_ {
            common_capnp::NodeType::BleKeyboard | common_capnp::NodeType::UsbKeyboard => {
                self.hidapi.serial_number.clone()
            }
            _ => self.serial.clone(),
        }
    }

    pub fn uid(&mut self) -> u64 {
        self.uid
    }

    pub fn created(&mut self) -> Instant {
        self.created
    }

    pub fn path(&mut self) -> String {
        self.hidapi.path.clone()
    }
}

/// Supported Ids by this module
#[cfg(feature = "api")]
pub fn supported_ids() -> Vec<HidIoCommandId> {
    capnp::supported_ids()
}

#[cfg(not(feature = "api"))]
pub fn supported_ids() -> Vec<HidIoCommandId> {
    vec![]
}

/// Cap'n'Proto API Initialization
/// Sets up a localhost socket to deal with localhost-only API usages
/// Some API usages may require external authentication to validate trustworthiness
#[cfg(feature = "api")]
pub async fn initialize(mailbox: mailbox::Mailbox) {
    capnp::initialize(mailbox).await;
}

#[cfg(not(feature = "api"))]
pub async fn initialize(_mailbox: mailbox::Mailbox) {}
