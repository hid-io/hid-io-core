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

pub mod uhid;

use crate::mailbox;
use std::sync::Arc;

/// USB VID:PID pairs for Virtual HID Devices
/// These are assigned by Input Club (as these are Input Club HID descriptors)
pub const IC_VID: u16 = 0x308F;
pub const IC_PID_KEYBOARD: u16 = 0x0030;
pub const IC_PID_MOUSE: u16 = 0x0031;

/// Standard 6KRO HID keyboard descriptor
/// Mirrors one used on Input Club keyboards
pub const KEYBOARD_6KRO: [u8; 68] = [
    // Keyboard Collection
    0x05, 0x01, //       Usage Page (Generic Desktop),
    0x09, 0x06, //       Usage (Keyboard),
    0xA1, 0x01, //       Collection (Application) - Keyboard,
    // Modifier Byte
    0x75, 0x01, //         Report Size (1),
    0x95, 0x08, //         Report Count (8),
    0x05, 0x07, //         Usage Page (Key Codes),
    0x15, 0x00, //         Logical Minimum (0),
    0x25, 0x01, //         Logical Maximum (1),
    0x19, 0xE0, //         Usage Minimum (224),
    0x29, 0xE7, //         Usage Maximum (231),
    0x81, 0x02, //         Input (Data, Variable, Absolute),
    // Reserved Byte
    0x75, 0x08, //         Report Size (8),
    0x95, 0x01, //         Report Count (1),
    0x81, 0x03, //         Input (Constant, Variable, Absolute),
    // LED Report
    0x75, 0x01, //         Report Size (1),
    0x95, 0x05, //         Report Count (5),
    0x05, 0x08, //         Usage Page (LEDs),
    0x15, 0x00, //         Logical Minimum (0),
    0x25, 0x01, //         Logical Maximum (1),
    0x19, 0x01, //         Usage Minimum (1),
    0x29, 0x05, //         Usage Maximum (5),
    0x91, 0x02, //         Output (Data, Variable, Absolute),
    // LED Report Padding
    0x75, 0x03, //         Report Size (3),
    0x95, 0x01, //         Report Count (1),
    0x91, 0x03, //         Output (Constant, Variable, Absolute),
    // Normal Keys
    0x75, 0x08, //         Report Size (8),
    0x95, 0x06, //         Report Count (6),
    0x05, 0x07, //         Usage Page (Key Codes),
    0x15, 0x00, //         Logical Minimum (0),
    0x26, 0xFF, 0x00, //   Logical Maximum (255), <-- Must be 16-bit send size (unsure why)
    0x19, 0x00, //         Usage Minimum (0),
    0x29, 0xFF, //         Usage Maximum (255),
    0x81, 0x00, //         Input (Data, Array),
    0xc0, //             End Collection - Keyboard
];

/// Input Club NKRO HID keyboard descriptor
/// Mirrors one used on Input Club keyboards
pub const KEYBOARD_NKRO: [u8; 95] = [
    // Keyboard Collection
    0x05, 0x01, //       Usage Page (Generic Desktop),
    0x09, 0x06, //       Usage (Keyboard),
    0xA1, 0x01, //       Collection (Application) - Keyboard,
    // LED Report
    0x75, 0x01, //         Report Size (1),
    0x95, 0x05, //         Report Count (5),
    0x05, 0x08, //         Usage Page (LEDs),
    0x15, 0x00, //         Logical Minimum (0),
    0x25, 0x01, //         Logical Maximum (1),
    0x19, 0x01, //         Usage Minimum (1),
    0x29, 0x05, //         Usage Maximum (5),
    0x91, 0x02, //         Output (Data, Variable, Absolute),
    // LED Report Padding
    0x75, 0x03, //         Report Size (3),
    0x95, 0x01, //         Report Count (1),
    0x91, 0x03, //         Output (Constant, Variable, Absolute),
    // Normal Keys - Using an NKRO Bitmap
    //
    // NOTES:
    // Supports all keys defined by the spec, except 1-3 which define error events
    //  and 0 which is "no keys pressed"
    // See http://www.usb.org/developers/hidpage/Hut1_12v2.pdf Chapter 10
    // Or Macros/PartialMap/usb_hid.h
    //
    // 165-175 are reserved/unused as well as 222-223 and 232-65535
    //
    // Compatibility Notes:
    //  - Using a second endpoint for a boot mode device helps with compatibility
    //  - DO NOT use Padding in the descriptor for bitfields
    //    (Mac OSX silently fails... Windows/Linux work correctly)
    //  - DO NOT use Report IDs, Windows 8.1 will not update keyboard correctly (modifiers disappear)
    //    (all other OSs, including OSX work fine...)
    //    (you can use them *iff* you only have 1 per collection)
    //  - Mac OSX and Windows 8.1 are extremely picky about padding
    //
    // Packing of bitmaps are as follows:
    //   4-164 : 21 bytes (0x04-0xA4) (161 bits + 4 padding bits + 3 padding bits for 21 bytes total)
    // 176-221 :  6 bytes (0xB0-0xDD) ( 46 bits + 2 padding bits for 6 bytes total)
    // 224-231 :  1 byte  (0xE0-0xE7) (  8 bits)

    // 224-231 (1 byte/8 bits) - Modifier Section
    0x75, 0x01, //         Report Size (1),
    0x95, 0x08, //         Report Count (8),
    0x15, 0x00, //         Logical Minimum (0),
    0x25, 0x01, //         Logical Maximum (1),
    0x05, 0x07, //         Usage Page (Key Codes),
    0x19, 0xE0, //         Usage Minimum (224),
    0x29, 0xE7, //         Usage Maximum (231),
    0x81, 0x02, //         Input (Data, Variable, Absolute, Bitfield),
    // Padding (4 bits)
    // Ignores Codes 0-3 (Keyboard Status codes)
    0x75, 0x04, //         Report Size (4),
    0x95, 0x01, //         Report Count (1),
    0x81, 0x03, //         Input (Constant),
    // 4-164 (21 bytes/161 bits + 4 bits + 3 bits) - Keyboard Section
    0x75, 0x01, //         Report Size (1),
    0x95, 0xA1, //         Report Count (161),
    0x15, 0x00, //         Logical Minimum (0),
    0x25, 0x01, //         Logical Maximum (1),
    0x05, 0x07, //         Usage Page (Key Codes),
    0x19, 0x04, //         Usage Minimum (4),
    0x29, 0xA4, //         Usage Maximum (164),
    0x81, 0x02, //         Input (Data, Variable, Absolute, Bitfield),
    // Padding (3 bits)
    0x75, 0x03, //         Report Size (3),
    0x95, 0x01, //         Report Count (1),
    0x81, 0x03, //         Input (Constant),
    // 176-221 (6 bytes/46 bits) - Keypad Section
    0x75, 0x01, //         Report Size (1),
    0x95, 0x2E, //         Report Count (46),
    0x15, 0x00, //         Logical Minimum (0),
    0x25, 0x01, //         Logical Maximum (1),
    0x05, 0x07, //         Usage Page (Key Codes),
    0x19, 0xB0, //         Usage Minimum (176),
    0x29, 0xDD, //         Usage Maximum (221),
    0x81, 0x02, //         Input (Data, Variable, Absolute, Bitfield),
    // Padding (2 bits)
    0x75, 0x02, //         Report Size (2),
    0x95, 0x01, //         Report Count (1),
    0x81, 0x03, //         Input (Constant),
    0xC0, //             End Collection - Keyboard
];

/// Input Club System Control + Consumer Control HID descriptor
/// Mirrors one used on Input Club keyboards
pub const SYSCTRL_CONSCTRL: [u8; 39] = [
    // Consumer Control Collection - Media Keys (16 bits)
    //
    // NOTES:
    // Not bothering with NKRO for this table. If there's a need, I can implement it. -HaaTa
    // Using a 1KRO scheme
    0x05, 0x0C, //       Usage Page (Consumer),
    0x09, 0x01, //       Usage (Consumer Control),
    0xA1, 0x01, //       Collection (Application),
    0x75, 0x10, //         Report Size (16),
    0x95, 0x01, //         Report Count (1),
    0x15, 0x00, //         Logical Minimum (0),
    0x26, 0x9D, 0x02, //   Logical Maximum (669),
    0x19, 0x00, //         Usage Minimum (0),
    0x2A, 0x9D, 0x02, //   Usage Maximum (669),
    0x81, 0x00, //         Input (Data, Array),
    // System Control Collection (8 bits)
    //
    // NOTES:
    // Not bothering with NKRO for this table. If there's need, I can implement it. -HaaTa
    // Using a 1KRO scheme
    0x05, 0x01, //         Usage Page (Generic Desktop),
    0x75, 0x08, //         Report Size (8),
    0x95, 0x01, //         Report Count (1),
    0x15, 0x01, //         Logical Minimum (1),
    //       ^-- Must start from 1 to resolve MS Windows problems
    0x25, 0x37, //         Logical Maximum (55),
    0x19, 0x81, //         Usage Minimum (129),
    //       ^<-- Must be 0x81/129 to fix macOS scrollbar issues
    0x29, 0xB7, //         Usage Maximum (183),
    0x81, 0x00, //         Input (Data, Array),
    0xC0, //             End Collection - Consumer Control
];

/// Input Club Mouse HID descriptor
/// Mirrors one used on Input Club keyboards
pub const MOUSE: [u8; 109] = [
    0x05, 0x01, //       Usage Page (Generic Desktop)
    0x09, 0x02, //       Usage (Mouse)
    0xA1, 0x01, //       Collection (Application)
    0x09, 0x01, //         Usage (Pointer)
    0xA1, 0x00, //         Collection (Physical)
    // Buttons (16 bits)
    0x05, 0x09, //           Usage Page (Button)
    0x19, 0x01, //           Usage Minimum (Button 1)
    0x29, 0x10, //           Usage Maximum (Button 16)
    0x15, 0x00, //           Logical Minimum (0)
    0x25, 0x01, //           Logical Maximum (1)
    0x75, 0x01, //           Report Size (1)
    0x95, 0x10, //           Report Count (16)
    0x81, 0x02, //           Input (Data,Var,Abs)
    // Pointer (32 bits)
    0x05, 0x01, //           Usage PAGE (Generic Desktop)
    0x09, 0x30, //           Usage (X)
    0x09, 0x31, //           Usage (Y)
    0x16, 0x01, 0x80, //     Logical Minimum (-32 767)
    0x26, 0xFF, 0x7F, //     Logical Maximum (32 767)
    0x75, 0x10, //           Report Size (16)
    0x95, 0x02, //           Report Count (2)
    0x81, 0x06, //           Input (Data,Var,Rel)
    // Vertical Wheel
    // - Multiplier (2 bits)
    0xA1, 0x02, //           Collection (Logical)
    0x09, 0x48, //             Usage (Resolution Multiplier)
    0x15, 0x00, //             Logical Minimum (0)
    0x25, 0x01, //             Logical Maximum (1)
    0x35, 0x01, //             Physical Minimum (1)
    0x45, 0x04, //             Physical Maximum (4)
    0x75, 0x02, //             Report Size (2)
    0x95, 0x01, //             Report Count (1)
    0xA4, //                   Push
    0xB1, 0x02, //             Feature (Data,Var,Abs)
    // - Device (8 bits)
    0x09, 0x38, //             Usage (Wheel)
    0x15, 0x81, //             Logical Minimum (-127)
    0x25, 0x7F, //             Logical Maximum (127)
    0x35, 0x00, //             Physical Minimum (0)        - reset physical
    0x45, 0x00, //             Physical Maximum (0)
    0x75, 0x08, //             Report Size (8)
    0x81, 0x06, //             Input (Data,Var,Rel)
    0xC0, //                 End Collection - Vertical Wheel
    // Horizontal Wheel
    // - Multiplier (2 bits)
    0xA1, 0x02, //           Collection (Logical)
    0x09, 0x48, //             Usage (Resolution Multiplier)
    0xB4, //                   Pop
    0xB1, 0x02, //             Feature (Data,Var,Abs)
    // - Padding (4 bits)
    0x35, 0x00, //             Physical Minimum (0)        - reset physical
    0x45, 0x00, //             Physical Maximum (0)
    0x75, 0x04, //             Report Size (4)
    0xB1, 0x03, //             Feature (Cnst,Var,Abs)
    // - Device (8 bits)
    0x05, 0x0C, //             Usage Page (Consumer Devices)
    0x0A, 0x38, 0x02, //       Usage (AC Pan)
    0x15, 0x81, //             Logical Minimum (-127)
    0x25, 0x7F, //             Logical Maximum (127)
    0x75, 0x08, //             Report Size (8)
    0x81, 0x06, //             Input (Data,Var,Rel)
    0xC0, //                 End Collection - Horizontal Wheel
    0xC0, //               End Collection - Mouse Physical
    0xC0, //             End Collection - Mouse Application
];

/// xbox 360 HID descriptor
pub const XBOX_360_CONTROLLER: [u8; 188] = [
    0x05, 0x01, //       USAGE_PAGE (Generic Desktop)
    0x09, 0x05, //       USAGE (Game Pad)
    0xa1, 0x01, //       COLLECTION (Application)
    0x05, 0x01, //         USAGE_PAGE (Generic Desktop)
    0x09, 0x3a, //         USAGE (Counted Buffer)
    0xa1, 0x02, //         COLLECTION (Logical)
    0x75, 0x08, //           REPORT_SIZE (8)
    0x95, 0x02, //           REPORT_COUNT (2)
    0x05, 0x01, //           USAGE_PAGE (Generic Desktop)
    0x09, 0x3f, //           USAGE (Reserved)
    0x09, 0x3b, //           USAGE (Byte Count)
    0x81, 0x01, //           INPUT (Cnst,Ary,Abs)
    0x75, 0x01, //           REPORT_SIZE (1)
    0x15, 0x00, //           LOGICAL_MINIMUM (0)
    0x25, 0x01, //           LOGICAL_MAXIMUM (1)
    0x35, 0x00, //           PHYSICAL_MINIMUM (0)
    0x45, 0x01, //           PHYSICAL_MAXIMUM (1)
    0x95, 0x04, //           REPORT_COUNT (4)
    0x05, 0x09, //           USAGE_PAGE (Button)
    0x19, 0x0c, //           USAGE_MINIMUM (Button 12)
    0x29, 0x0f, //           USAGE_MAXIMUM (Button 15)
    0x81, 0x02, //           INPUT (Data,Var,Abs)
    0x75, 0x01, //           REPORT_SIZE (1)
    0x15, 0x00, //           LOGICAL_MINIMUM (0)
    0x25, 0x01, //           LOGICAL_MAXIMUM (1)
    0x35, 0x00, //           PHYSICAL_MINIMUM (0)
    0x45, 0x01, //           PHYSICAL_MAXIMUM (1)
    0x95, 0x04, //           REPORT_COUNT (4)
    0x05, 0x09, //           USAGE_PAGE (Button)
    0x09, 0x09, //           USAGE (Button 9)
    0x09, 0x0a, //           USAGE (Button 10)
    0x09, 0x07, //           USAGE (Button 7)
    0x09, 0x08, //           USAGE (Button 8)
    0x81, 0x02, //           INPUT (Data,Var,Abs)
    0x75, 0x01, //           REPORT_SIZE (1)
    0x15, 0x00, //           LOGICAL_MINIMUM (0)
    0x25, 0x01, //           LOGICAL_MAXIMUM (1)
    0x35, 0x00, //           PHYSICAL_MINIMUM (0)
    0x45, 0x01, //           PHYSICAL_MAXIMUM (1)
    0x95, 0x03, //           REPORT_COUNT (3)
    0x05, 0x09, //           USAGE_PAGE (Button)
    0x09, 0x05, //           USAGE (Button 5)
    0x09, 0x06, //           USAGE (Button 6)
    0x09, 0x0b, //           USAGE (Button 11)
    0x81, 0x02, //           INPUT (Data,Var,Abs)
    0x75, 0x01, //           REPORT_SIZE (1)
    0x95, 0x01, //           REPORT_COUNT (1)
    0x81, 0x01, //           INPUT (Cnst,Ary,Abs)
    0x75, 0x01, //           REPORT_SIZE (1)
    0x15, 0x00, //           LOGICAL_MINIMUM (0)
    0x25, 0x01, //           LOGICAL_MAXIMUM (1)
    0x35, 0x00, //           PHYSICAL_MINIMUM (0)
    0x45, 0x01, //           PHYSICAL_MAXIMUM (1)
    0x95, 0x04, //           REPORT_COUNT (4)
    0x05, 0x09, //           USAGE_PAGE (Button)
    0x19, 0x01, //           USAGE_MINIMUM (Button 1)
    0x29, 0x04, //           USAGE_MAXIMUM (Button 4)
    0x81, 0x02, //           INPUT (Data,Var,Abs)
    0x75, 0x08, //           REPORT_SIZE (8)
    0x15, 0x00, //           LOGICAL_MINIMUM (0)
    0x26, 0xff, 0x00, //     LOGICAL_MAXIMUM (255)
    0x35, 0x00, //           PHYSICAL_MINIMUM (0)
    0x46, 0xff, 0x00, //     PHYSICAL_MAXIMUM (255)
    0x95, 0x02, //           REPORT_COUNT (2)
    0x05, 0x01, //           USAGE_PAGE (Generic Desktop)
    0x09, 0x32, //           USAGE (Z)
    0x09, 0x35, //           USAGE (Rz)
    0x81, 0x02, //           INPUT (Data,Var,Abs)
    0x75, 0x10, //           REPORT_SIZE (16)
    0x16, 0x00, 0x80, //     LOGICAL_MINIMUM (-32768)
    0x26, 0xff, 0x7f, //     LOGICAL_MAXIMUM (32767)
    0x36, 0x00, 0x80, //     PHYSICAL_MINIMUM (-32768)
    0x46, 0xff, 0x7f, //     PHYSICAL_MAXIMUM (32767)
    0x05, 0x01, //           USAGE_PAGE (Generic Desktop)
    0x09, 0x01, //           USAGE (Pointer)
    0xa1, 0x00, //           COLLECTION (Physical)
    0x95, 0x02, //             REPORT_COUNT (2)
    0x05, 0x01, //             USAGE_PAGE (Generic Desktop)
    0x09, 0x30, //             USAGE (X)
    0x09, 0x31, //             USAGE (Y)
    0x81, 0x02, //             INPUT (Data,Var,Abs)
    0xc0, //                 END_COLLECTION
    0x05, 0x01, //           USAGE_PAGE (Generic Desktop)
    0x09, 0x01, //           USAGE (Pointer)
    0xa1, 0x00, //           COLLECTION (Physical)
    0x95, 0x02, //             REPORT_COUNT (2)
    0x05, 0x01, //             USAGE_PAGE (Generic Desktop)
    0x09, 0x33, //             USAGE (Rx)
    0x09, 0x34, //             USAGE (Ry)
    0x81, 0x02, //             INPUT (Data,Var,Abs)
    0xc0, //                 END_COLLECTION
    0xc0, //               END_COLLECTION
    0xc0, //             END_COLLECTION
];

/// HID-IO RawHID (rawio) HID descriptor
/// Mirrors one used on Input Club devices
pub const RAWIO: [u8; 28] = [
    0x06, 0x1C, 0xFF, // Usage Page (Vendor Defined) 0xFF1C
    0x0A, 0x00, 0x11, // Usage (0x1100)
    0xA1, 0x01, //       Collection (Application)
    0x75, 0x08, //         Report Size (8)
    0x15, 0x00, //         Logical Minimum (0)
    0x26, 0xFF, 0x00, //   Logical Maximum (255)
    0x95, 0x40, //           Report Count (64 bytes)
    0x09, 0x01, //           Usage (Output)
    0x91, 0x02, //           Output (Data,Var,Abs)
    0x95, 0x40, //           Report Count (64 bytes)
    0x09, 0x02, //           Usage (Input)
    0x81, 0x02, //           Input (Data,Var,Abs)
    0xC0, //             End Collection
];

/// vhid initialization
/// Handles setting up the vhid interface
/// Depending on the platform, there may be support for dynamically created/configured hid devices
/// (tbd)
#[cfg(target_os = "linux")]
pub async fn initialize(rt: Arc<tokio::runtime::Runtime>, mailbox: mailbox::Mailbox) {
    info!("Initializing module/vhid...");

    // Initialize the platform specific module
    #[cfg(target_os = "linux")]
    uhid::initialize(rt, mailbox).await;
}

#[cfg(target_os = "macos")]
pub async fn initialize(_rt: Arc<tokio::runtime::Runtime>, mailbox: mailbox::Mailbox) {
    info!("Initializing module/vhid...");

    // Initialize the platform specific module
    // TODO
}

#[cfg(target_os = "windows")]
pub async fn initialize(_rt: Arc<tokio::runtime::Runtime>, mailbox: mailbox::Mailbox) {
    info!("Initializing module/vhid...");

    // Initialize the platform specific module
    // TODO
}
