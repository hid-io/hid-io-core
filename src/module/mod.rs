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
pub mod daemonnode;
pub mod displayserver;
pub mod vhid;

use crate::api;
use crate::device;
use crate::mailbox;
use crate::protocol::hidio::*;
use std::sync::Arc;
use tokio::stream::StreamExt;

fn as_u8_slice(v: &[u16]) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(
            v.as_ptr() as *const u8,
            v.len() * std::mem::size_of::<u16>(),
        )
    }
}

/// Supported Ids by this module
/// recursive option applies supported ids from child modules as well
pub fn supported_ids(recursive: bool) -> Vec<HidIoCommandID> {
    let mut ids = vec![
        HidIoCommandID::SupportedIDs,
        HidIoCommandID::GetProperties,
        HidIoCommandID::OpenURL,
    ];
    if recursive {
        ids.extend(displayserver::supported_ids().iter().cloned());
    }
    ids
}

/// Device initialization
/// Sets up a scanning thread per Device type (using tokio).
/// Each scanning thread will create a new thread per device found.
/// The scanning thread is required in case devices are plugged/unplugged while running.
/// If a device is unplugged, the Device thread will exit.
pub async fn initialize(rt: Arc<tokio::runtime::Runtime>, mailbox: mailbox::Mailbox) {
    info!("Initializing modules...");

    // Setup local thread
    // Due to some of the setup in the Module struct we need to run processing in the same local
    // thread.
    let mailbox1 = mailbox.clone();
    let data = tokio::spawn(async move {
        // Setup receiver stream
        let sender = mailbox1.clone().sender.clone();
        let receiver = sender.clone().subscribe();
        tokio::pin! {
            let stream = receiver.into_stream()
                .filter(Result::is_ok).map(Result::unwrap)
                .take_while(|msg|
                    msg.src != mailbox::Address::DropSubscription &&
                    msg.dst != mailbox::Address::CancelAllSubscriptions
                )
                .filter(|msg| msg.dst == mailbox::Address::Module)
                .filter(|msg| supported_ids(false).contains(&msg.data.id))
                .filter(|msg| msg.data.ptype == HidIoPacketType::Data || msg.data.ptype == HidIoPacketType::NAData);
        }

        // Process filtered message stream
        while let Some(msg) = stream.next().await {
            let mydata = msg.data.data.clone();
            debug!("Processing command: {:?}", msg.data.id);
            match msg.data.id {
                HidIoCommandID::SupportedIDs => {
                    let ids = supported_ids(false)
                        .iter()
                        .map(|x| *x as u16)
                        .collect::<Vec<u16>>();
                    msg.send_ack(sender.clone(), as_u8_slice(&ids).to_vec());
                }
                HidIoCommandID::GetProperties => {
                    use crate::built_info;
                    let property: HidIoPropertyID = unsafe { std::mem::transmute(mydata[0]) };
                    info!("Get prop {:?}", property);
                    match property {
                        HidIoPropertyID::HidIoMajor => {
                            let v = built_info::PKG_VERSION_MAJOR.parse::<u16>().unwrap();
                            msg.send_ack(sender.clone(), as_u8_slice(&[v]).to_vec());
                        }
                        HidIoPropertyID::HidIoMinor => {
                            let v = built_info::PKG_VERSION_MINOR.parse::<u16>().unwrap();
                            msg.send_ack(sender.clone(), as_u8_slice(&[v]).to_vec());
                        }
                        HidIoPropertyID::HidIoPatch => {
                            let v = built_info::PKG_VERSION_PATCH.parse::<u16>().unwrap();
                            msg.send_ack(sender.clone(), as_u8_slice(&[v]).to_vec());
                        }
                        HidIoPropertyID::HostOS => {
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
                        HidIoPropertyID::OSVersion => match sys_info::os_release() {
                            Ok(version) => {
                                msg.send_ack(sender.clone(), version.as_bytes().to_vec());
                            }
                            Err(e) => {
                                error!("OS Release retrieval failed: {}", e);
                                msg.send_nak(sender.clone(), vec![]);
                            }
                        },
                        HidIoPropertyID::HostName => {
                            let name = built_info::PKG_NAME;
                            msg.send_ack(sender.clone(), name.as_bytes().to_vec());
                        }
                    };
                }
                HidIoCommandID::HostMacro => {
                    warn!("Host Macro not implemented");
                    msg.send_nak(sender.clone(), vec![]);
                }
                HidIoCommandID::KLLState => {
                    warn!("KLL State not implemented");
                    msg.send_nak(sender.clone(), vec![]);
                }
                HidIoCommandID::OpenURL => {
                    let s = String::from_utf8(mydata).unwrap();
                    println!("Open url: {}", s);
                    open::that(s).unwrap();
                    msg.send_ack(sender.clone(), vec![]);
                }
                _ => {}
            }
        }
    });

    // NAK unsupported command ids
    let mailbox2 = mailbox.clone();
    let naks = tokio::spawn(async move {
        // Setup receiver stream
        let sender = mailbox2.clone().sender.clone();
        let receiver = sender.clone().subscribe();
        tokio::pin! {
            let stream = receiver.into_stream()
                .filter(Result::is_ok).map(Result::unwrap)
                .take_while(|msg|
                    msg.src != mailbox::Address::DropSubscription &&
                    msg.dst != mailbox::Address::CancelAllSubscriptions
                )
                .filter(|msg| !(
                    supported_ids(true).contains(&msg.data.id) ||
                    api::supported_ids().contains(&msg.data.id) ||
                    device::supported_ids(true).contains(&msg.data.id)
                ))
                .filter(|msg| msg.data.ptype == HidIoPacketType::Data || msg.data.ptype == HidIoPacketType::NAData);
        }

        // Process filtered message stream
        while let Some(msg) = stream.next().await {
            warn!("Unknown command ID: {:?} ({})", msg.data.id, msg.data.ptype);
            // Only send NAK with Data packets (NAData packets don't have acknowledgements, so just
            // warn)
            if msg.data.ptype == HidIoPacketType::Data {
                msg.send_nak(sender.clone(), vec![]);
            }
        }
    });

    let (_, _, _, _, _) = tokio::join!(
        daemonnode::initialize(rt.clone(), mailbox.clone()),
        displayserver::initialize(rt.clone(), mailbox.clone()),
        naks,
        data,
        vhid::initialize(rt.clone(), mailbox.clone()),
    );
}
