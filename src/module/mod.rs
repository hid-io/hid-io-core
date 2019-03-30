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

/// Platform specific character output and IME control
pub mod unicode;

/// Host-Side KLL
pub mod kll;

use crate::device::*;
use crate::module::unicode::*;
use crate::protocol::hidio::*;
use crate::RUNNING;

#[cfg(all(feature = "unicode", target_os = "linux"))]
use crate::module::unicode::x11::*;

#[cfg(all(feature = "unicode", target_os = "windows"))]
use crate::module::unicode::winapi::*;

#[cfg(all(feature = "unicode", target_os = "macos"))]
use crate::module::unicode::osx::*;

use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;

const PROCESS_DELAY: u64 = 1;

// TODO: Use const fn to adjust based on cago features
// TODO: Let capnp nodes add to this list
/// List of commands we advertise as supporting to devices
pub const SUPPORTED_IDS: &[HIDIOCommandID] = &[
    HIDIOCommandID::SupportedIDs,
    HIDIOCommandID::GetProperties,
    HIDIOCommandID::UnicodeText,
    HIDIOCommandID::UnicodeKey,
    HIDIOCommandID::OpenURL,
    HIDIOCommandID::Terminal,
];

fn as_u8_slice(v: &[u16]) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(
            v.as_ptr() as *const u8,
            v.len() * std::mem::size_of::<u16>(),
        )
    }
}

/// Our "internal" node responsible for handling required commands
struct HIDIOHandler {
    mailbox: HIDIOMailbox,
    display: Box<UnicodeOutput>,
}

#[cfg(not(feature = "unicode"))]
fn get_display() -> Box<UnicodeOutput> {
    Box::new(StubOutput::new())
}

#[cfg(all(feature = "unicode", target_os = "linux"))]
fn get_display() -> Box<UnicodeOutput> {
    Box::new(XConnection::new())
}

#[cfg(all(feature = "unicode", target_os = "windows"))]
fn get_display() -> Box<UnicodeOutput> {
    Box::new(DisplayConnection::new())
}

#[cfg(all(feature = "unicode", target_os = "macos"))]
fn get_display() -> Box<UnicodeOutput> {
    Box::new(OSXConnection::new())
}

impl HIDIOHandler {
    fn new(mailbox: HIDIOMailbox) -> HIDIOHandler {
        let connection = get_display();

        let layout = connection.get_layout();
        info!("Current layout: {}", layout);

        HIDIOHandler {
            mailbox,
            display: connection,
        }
    }

    /// hidusb device processing
    /// Will handle all messages we support.
    /// The capnp api can be used to add extended message types by 3rd party programs
    fn process(&mut self) {
        let mailbox = &self.mailbox;

        loop {
            if !RUNNING.load(Ordering::SeqCst) {
                break;
            }
            let message = mailbox.recv_psuedoblocking();
            if message.is_none() {
                continue;
            }
            let message = message.unwrap();

            let (device, received) = (message.device, message.message);

            if received.ptype != HIDIOPacketType::Data {
                continue;
            }

            let id: HIDIOCommandID = unsafe { std::mem::transmute(received.id as u16) };
            let mydata = received.data.clone();
            //info!("Processing command: {:?}", id);
            match id {
                HIDIOCommandID::SupportedIDs => {
                    let ids = SUPPORTED_IDS
                        .iter()
                        .map(|x| unsafe { std::mem::transmute(*x) })
                        .collect::<Vec<u16>>();
                    mailbox.send_ack(device, received.id, as_u8_slice(&ids).to_vec());
                }
                HIDIOCommandID::GetProperties => {
                    use crate::built_info;
                    let property: HIDIOPropertyID = unsafe { std::mem::transmute(mydata[0]) };
                    info!("Get prop {:?}", property);
                    match property {
                        HIDIOPropertyID::HIDIOMajor => {
                            let v = built_info::PKG_VERSION_MAJOR.parse::<u16>().unwrap();
                            mailbox.send_ack(device, received.id, as_u8_slice(&[v]).to_vec());
                        }
                        HIDIOPropertyID::HIDIOMinor => {
                            let v = built_info::PKG_VERSION_MINOR.parse::<u16>().unwrap();
                            mailbox.send_ack(device, received.id, as_u8_slice(&[v]).to_vec());
                        }
                        HIDIOPropertyID::HIDIOPatch => {
                            let v = built_info::PKG_VERSION_PATCH.parse::<u16>().unwrap();
                            mailbox.send_ack(device, received.id, as_u8_slice(&[v]).to_vec());
                        }
                        HIDIOPropertyID::HostOS => {
                            let os = match built_info::CFG_OS {
                                "windows" => HostOSID::Windows,
                                "macos" => HostOSID::Mac,
                                "ios" => HostOSID::IOS,
                                "linux" => HostOSID::Linux,
                                "android" => HostOSID::Android,
                                "freebsd" | "openbsd" | "netbsd" => HostOSID::Linux,
                                _ => HostOSID::Unknown,
                            };
                            mailbox.send_ack(device, received.id, vec![os as u8]);
                        }
                        HIDIOPropertyID::OSVersion => {
                            let version = "1.0"; // TODO: Retreive in cross platform way
                            mailbox.send_ack(device, received.id, version.as_bytes().to_vec());
                        }
                        HIDIOPropertyID::HostName => {
                            let name = built_info::PKG_NAME;
                            mailbox.send_ack(device, received.id, name.as_bytes().to_vec());
                        }
                        HIDIOPropertyID::InputLayout => {
                            let layout = self.display.get_layout();
                            println!("Current layout: {}", layout);
                            mailbox.send_ack(device, received.id, layout.as_bytes().to_vec());
                        }
                    };
                }
                HIDIOCommandID::UnicodeText => {
                    let s = String::from_utf8(mydata).unwrap();
                    self.display.type_string(&s);
                    mailbox.send_ack(device, received.id, vec![]);
                }
                HIDIOCommandID::UnicodeKey => {
                    let s = String::from_utf8(mydata).unwrap();
                    self.display.set_held(&s);
                    mailbox.send_ack(device, received.id, vec![]);
                }
                HIDIOCommandID::HostMacro => {
                    warn!("TODO");
                    mailbox.send_ack(device, received.id, vec![]);
                }
                HIDIOCommandID::KLLState => {
                    warn!("TODO");
                    mailbox.send_ack(device, received.id, vec![]);
                }
                HIDIOCommandID::OpenURL => {
                    let s = String::from_utf8(mydata).unwrap();
                    println!("Open url: {}", s);
                    open::that(s).unwrap();
                    mailbox.send_ack(device, received.id, vec![]);
                }
                HIDIOCommandID::Terminal => {
                    mailbox.send_ack(device, received.id, vec![]);
                    /*std::io::stdout().write_all(&mydata).unwrap();
                    std::io::stdout().flush().unwrap();*/
                }
                HIDIOCommandID::InputLayout => {
                    let s = String::from_utf8(mydata).unwrap();
                    info!("Setting language to {}", s);
                }
                _ => {
                    warn!("Unknown command ID: {:?}", &received.id);
                    mailbox.send_nack(device, received.id, vec![]);
                }
            }

            thread::sleep(Duration::from_millis(PROCESS_DELAY));
        }
    }
}

/// Device initialization
/// Sets up a scanning thread per Device type.
/// Each scanning thread will create a new thread per device found.
/// The scanning thread is required in case devices are plugged/unplugged while running.
/// If a device is unplugged, the Device thread will exit.
pub fn initialize(mailbox: HIDIOMailbox) -> std::thread::JoinHandle<()> {
    info!("Initializing modules...");

    thread::Builder::new()
        .name("Command Handler".to_string())
        .spawn(move || {
            let mut handler = HIDIOHandler::new(mailbox);
            handler.process();
        })
        .unwrap()
}
