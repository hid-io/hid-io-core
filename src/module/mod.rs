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

/// Platform specific character output and IME control
pub mod unicode;

use crate::mailbox;
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
use tokio::stream::StreamExt;

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
struct Module {
    display: Box<dyn UnicodeOutput>,
}

#[cfg(not(feature = "unicode"))]
fn get_display() -> Box<dyn UnicodeOutput> {
    Box::new(StubOutput::new())
}

#[cfg(all(feature = "unicode", target_os = "linux"))]
fn get_display() -> Box<dyn UnicodeOutput> {
    Box::new(XConnection::new())
}

#[cfg(all(feature = "unicode", target_os = "windows"))]
fn get_display() -> Box<dyn UnicodeOutput> {
    Box::new(DisplayConnection::new())
}

#[cfg(all(feature = "unicode", target_os = "macos"))]
fn get_display() -> Box<dyn UnicodeOutput> {
    Box::new(OSXConnection::new())
}

impl Module {
    fn new() -> Module {
        let connection = get_display();

        let layout = connection.get_layout();
        info!("Current layout: {}", layout);

        Module {
            display: connection,
        }
    }
}

/// Device initialization
/// Sets up a scanning thread per Device type (using tokio).
/// Each scanning thread will create a new thread per device found.
/// The scanning thread is required in case devices are plugged/unplugged while running.
/// If a device is unplugged, the Device thread will exit.
pub async fn initialize(mailbox: mailbox::Mailbox) {
    info!("Initializing modules...");

    // Setup local thread
    // Due to some of the setup in the Module struct we need to run processing in the same local
    // thread.
    let local = tokio::task::LocalSet::new();
    local.spawn_local(async move {

        // Top-level module setup
        let mut module = Module::new();

        // Setup receiver stream
        let sender = mailbox.clone().sender.clone();
        let receiver = sender.clone().subscribe();
        tokio::pin! {
            let stream = receiver.into_stream().filter(Result::is_ok).map(Result::unwrap).filter(|msg| msg.src == mailbox::Address::Module);
        }

        // Process filtered message stream
        while let Some(msg) = stream.next().await {
            info!("My msg2: {} {:?} {:?}", msg.data, msg.src, msg.dst); // TODO REMOVEME

            // Make sure this is a valid packet
            if msg.data.ptype != HIDIOPacketType::Data {
                continue;
            }

            let id: HIDIOCommandID = unsafe { std::mem::transmute(msg.data.id as u16) };
            let mydata = msg.data.data.clone();
            debug!("Processing command: {:?}", id);
            match id {
                HIDIOCommandID::SupportedIDs => {
                    let ids = SUPPORTED_IDS
                        .iter()
                        .map(|x| unsafe { std::mem::transmute(*x) })
                        .collect::<Vec<u16>>();
                    msg.send_ack(sender.clone(), as_u8_slice(&ids).to_vec());
                }
                HIDIOCommandID::GetProperties => {
                    use crate::built_info;
                    let property: HIDIOPropertyID = unsafe { std::mem::transmute(mydata[0]) };
                    info!("Get prop {:?}", property);
                    match property {
                        HIDIOPropertyID::HIDIOMajor => {
                            let v = built_info::PKG_VERSION_MAJOR.parse::<u16>().unwrap();
                            msg.send_ack(sender.clone(), as_u8_slice(&[v]).to_vec());
                        }
                        HIDIOPropertyID::HIDIOMinor => {
                            let v = built_info::PKG_VERSION_MINOR.parse::<u16>().unwrap();
                            msg.send_ack(sender.clone(), as_u8_slice(&[v]).to_vec());
                        }
                        HIDIOPropertyID::HIDIOPatch => {
                            let v = built_info::PKG_VERSION_PATCH.parse::<u16>().unwrap();
                            msg.send_ack(sender.clone(), as_u8_slice(&[v]).to_vec());
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
                            msg.send_ack(sender.clone(), vec![os as u8]);
                        }
                        HIDIOPropertyID::OSVersion => {
                            let version = "1.0"; // TODO: Retreive in cross platform way
                            msg.send_ack(sender.clone(), version.as_bytes().to_vec());
                        }
                        HIDIOPropertyID::HostName => {
                            let name = built_info::PKG_NAME;
                            msg.send_ack(sender.clone(), name.as_bytes().to_vec());
                        }
                        HIDIOPropertyID::InputLayout => {
                            let layout = module.display.get_layout();
                            println!("Current layout: {}", layout);
                            msg.send_ack(sender.clone(), layout.as_bytes().to_vec());
                        }
                    };
                }
                HIDIOCommandID::UnicodeText => {
                    let s = String::from_utf8(mydata).unwrap();
                    module.display.type_string(&s);
                    msg.send_ack(sender.clone(), vec![]);
                }
                HIDIOCommandID::UnicodeKey => {
                    let s = String::from_utf8(mydata).unwrap();
                    module.display.set_held(&s);
                    msg.send_ack(sender.clone(), vec![]);
                }
                HIDIOCommandID::HostMacro => {
                    warn!("TODO");
                    msg.send_ack(sender.clone(), vec![]);
                }
                HIDIOCommandID::KLLState => {
                    warn!("TODO");
                    msg.send_ack(sender.clone(), vec![]);
                }
                HIDIOCommandID::OpenURL => {
                    let s = String::from_utf8(mydata).unwrap();
                    println!("Open url: {}", s);
                    open::that(s).unwrap();
                    msg.send_ack(sender.clone(), vec![]);
                }
                HIDIOCommandID::Terminal => {
                    msg.send_ack(sender.clone(), vec![]);
                    /*std::io::stdout().write_all(&mydata).unwrap();
                    std::io::stdout().flush().unwrap();*/
                }
                HIDIOCommandID::InputLayout => {
                    let s = String::from_utf8(mydata).unwrap();
                    info!("Setting language to {}", s);
                }
                _ => {
                    warn!("Unknown command ID: {:?}", msg.data.id);
                    msg.send_nak(sender.clone(), vec![]);
                }
            }
        }
    });

    // Wait for exit signal before cleaning up
    local
        .run_until(async move {
            loop {
                if !RUNNING.load(Ordering::SeqCst) {
                    break;
                }
                tokio::time::delay_for(std::time::Duration::from_millis(100)).await;
            }
        })
        .await;
}
