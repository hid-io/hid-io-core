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

/// Platform specific character output and IME control
pub mod daemonnode;
pub mod displayserver;
pub mod vhid;

use crate::api;
use crate::built_info;
use crate::device;
use crate::mailbox;
use hid_io_protocol::commands::*;
use hid_io_protocol::{HidIoCommandId, HidIoPacketType};
use tokio_stream::{wrappers::BroadcastStream, StreamExt};

/// Max number of commands supported by this hid-io-core processor
/// can be increased as necessary.
const CMD_SIZE: usize = 200;

/// hid-io-protocol CommandInterface for top-level module
/// Used to serialize the Ack packets before sending them through the mailbox
struct CommandInterface {
    src: mailbox::Address,
    dst: mailbox::Address,
    mailbox: mailbox::Mailbox,
}

impl
    Commands<
        { mailbox::HIDIO_PKT_BUF_DATA_SIZE },
        { mailbox::HIDIO_PKT_BUF_DATA_SIZE - 1 },
        { mailbox::HIDIO_PKT_BUF_DATA_SIZE - 2 },
        { mailbox::HIDIO_PKT_BUF_DATA_SIZE - 4 },
        CMD_SIZE,
    > for CommandInterface
{
    fn tx_packetbuffer_send(
        &mut self,
        buf: &mut mailbox::HidIoPacketBuffer,
    ) -> Result<(), CommandError> {
        if let Some(rcvmsg) = self.mailbox.try_send_message(mailbox::Message {
            src: self.src,
            dst: self.dst,
            data: buf.clone(),
        })? {
            // Handle ack/nak
            self.rx_message_handling(rcvmsg.data)?;
        }
        Ok(())
    }

    fn h0000_supported_ids_cmd(
        &mut self,
        _data: h0000::Cmd,
    ) -> Result<h0000::Ack<CMD_SIZE>, h0000::Nak> {
        let ids = heapless::Vec::from_slice(&crate::supported_ids()).unwrap();
        Ok(h0000::Ack { ids })
    }

    fn h0001_info_cmd(
        &mut self,
        data: h0001::Cmd,
    ) -> Result<h0001::Ack<{ mailbox::HIDIO_PKT_BUF_DATA_SIZE - 1 }>, h0001::Nak> {
        let mut ack = h0001::Ack::<{ mailbox::HIDIO_PKT_BUF_DATA_SIZE - 1 }> {
            property: data.property,
            os: h0001::OsType::Unknown,
            number: 0,
            string: heapless::String::from(""),
        };
        match data.property {
            h0001::Property::MajorVersion => {
                ack.number = built_info::PKG_VERSION_MAJOR.parse::<u16>().unwrap();
            }
            h0001::Property::MinorVersion => {
                ack.number = built_info::PKG_VERSION_MINOR.parse::<u16>().unwrap();
            }
            h0001::Property::PatchVersion => {
                ack.number = built_info::PKG_VERSION_PATCH.parse::<u16>().unwrap();
            }
            h0001::Property::OsType => {
                ack.os = match built_info::CFG_OS {
                    "windows" => h0001::OsType::Windows,
                    "macos" => h0001::OsType::MacOs,
                    "ios" => h0001::OsType::Ios,
                    "linux" => h0001::OsType::Linux,
                    "android" => h0001::OsType::Android,
                    "freebsd" => h0001::OsType::FreeBsd,
                    "openbsd" => h0001::OsType::OpenBsd,
                    "netbsd" => h0001::OsType::NetBsd,
                    _ => h0001::OsType::Unknown,
                };
            }
            h0001::Property::OsVersion => match sys_info::os_release() {
                Ok(version) => {
                    ack.string = heapless::String::from(version.as_str());
                }
                Err(e) => {
                    error!("OS Release retrieval failed: {}", e);
                    return Err(h0001::Nak {
                        property: h0001::Property::OsVersion,
                    });
                }
            },
            h0001::Property::HostSoftwareName => {
                ack.string = heapless::String::from(built_info::PKG_NAME);
            }
            _ => {
                return Err(h0001::Nak {
                    property: h0001::Property::Unknown,
                });
            }
        }
        Ok(ack)
    }

    fn h0030_openurl_cmd(
        &mut self,
        data: h0030::Cmd<{ mailbox::HIDIO_PKT_BUF_DATA_SIZE }>,
    ) -> Result<h0030::Ack, h0030::Nak> {
        debug!("Open url: {}", data.url);
        let url = String::from(data.url.as_str());
        if let Err(err) = open::that(url.clone()) {
            error!("Failed to open url: {:?} - {:?}", url, err);
            Err(h0030::Nak {})
        } else {
            Ok(h0030::Ack {})
        }
    }
}

/// Supported Ids by this module
/// recursive option applies supported ids from child modules as well
pub fn supported_ids(recursive: bool) -> Vec<HidIoCommandId> {
    let mut ids = vec![
        HidIoCommandId::GetInfo,
        HidIoCommandId::OpenUrl,
        HidIoCommandId::SupportedIds,
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
pub async fn initialize(mailbox: mailbox::Mailbox) {
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
            let stream = BroadcastStream::new(receiver)
                .filter(Result::is_ok).map(Result::unwrap)
                .take_while(|msg|
                    msg.src != mailbox::Address::DropSubscription &&
                    msg.dst != mailbox::Address::CancelAllSubscriptions
                )
                .filter(|msg| msg.dst == mailbox::Address::Module || msg.dst == mailbox::Address::All)
                .filter(|msg| supported_ids(false).contains(&msg.data.id))
                .filter(|msg| msg.data.ptype == HidIoPacketType::Data || msg.data.ptype == HidIoPacketType::NaData);
        }

        // Process filtered message stream
        while let Some(msg) = stream.next().await {
            // Process buffer using hid-io-protocol
            let mut intf = CommandInterface {
                src: msg.dst, // Replying to message
                dst: msg.src, // Replying to message
                mailbox: mailbox1.clone(),
            };
            if let Err(err) = intf.rx_message_handling(msg.clone().data) {
                warn!("Failed to process({:?}): {:?}", err, msg);
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
            let stream = BroadcastStream::new(receiver)
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
                .filter(|msg| msg.data.ptype == HidIoPacketType::Data || msg.data.ptype == HidIoPacketType::NaData);
        }

        // Process filtered message stream
        while let Some(msg) = stream.next().await {
            warn!("Unknown command ID: {:?} ({})", msg.data.id, msg.data.ptype);
            // Only send NAK with Data packets (NaData packets don't have acknowledgements, so just
            // warn)
            if msg.data.ptype == HidIoPacketType::Data {
                msg.send_nak(sender.clone(), vec![]);
            }
        }
    });

    let (_, _, _, _, _) = tokio::join!(
        daemonnode::initialize(mailbox.clone()),
        displayserver::initialize(mailbox.clone()),
        naks,
        data,
        vhid::initialize(mailbox.clone()),
    );
}

/// Used when displayserver feature is disabled
#[cfg(not(feature = "displayserver"))]
mod displayserver {
    use crate::mailbox;
    use hid_io_protocol::HidIoCommandId;

    pub async fn initialize(_mailbox: mailbox::Mailbox) {}
    pub fn supported_ids() -> Vec<HidIoCommandId> {
        vec![]
    }
}

/// Used when displayserver feature is disabled
#[cfg(not(feature = "dev-capture"))]
mod vhid {
    use crate::mailbox;

    pub async fn initialize(_mailbox: mailbox::Mailbox) {}
}
