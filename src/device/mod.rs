/* Copyright (C) 2017-2021 by Jacob Alexander
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

pub mod evdev;
pub mod hidapi;

/// Handles hidapi devices
///
/// Works with both USB and BLE HID devices
use crate::mailbox;
use hid_io_protocol::*;
use std::io::{Read, Write};
use std::time::Instant;
use tokio::sync::broadcast;

/// A duplex stream for HidIo to communicate over
pub trait HidIoTransport: Read + Write {}

const MAX_RECV_SIZE: usize = 1024;

/// A raw transport plus any associated metadata
///
/// Contains helpers to encode/decode HidIo packets
pub struct HidIoEndpoint {
    socket: Box<dyn HidIoTransport>,
    max_packet_len: u32,
}

impl HidIoEndpoint {
    pub fn new(socket: Box<dyn HidIoTransport>, max_packet_len: u32) -> HidIoEndpoint {
        HidIoEndpoint {
            socket,
            max_packet_len,
        }
    }

    pub fn recv_chunk(
        &mut self,
        buffer: &mut mailbox::HidIoPacketBuffer,
    ) -> Result<usize, std::io::Error> {
        let mut rbuf = [0; MAX_RECV_SIZE];
        match self.socket.read(&mut rbuf) {
            Ok(len) => {
                if len > 0 {
                    let slice = &rbuf[0..len];
                    let ret = buffer.decode_packet(slice);
                    if let Err(e) = ret {
                        error!("recv_chunk({}) {:?}", len, e);
                        println!("received: {:?}", slice);
                        println!("current state: {:?}", buffer);
                        std::process::exit(2);
                    } else {
                        debug!("R{} {:x?}", buffer.data.len(), buffer);
                    }
                }

                Ok(len)
            }
            Err(e) => Err(e),
        }
    }

    pub fn create_buffer(&self) -> mailbox::HidIoPacketBuffer {
        let mut buffer = HidIoPacketBuffer::new();
        buffer.max_len = self.max_packet_len;
        buffer
    }

    pub fn send_packet(
        &mut self,
        packet: mailbox::HidIoPacketBuffer,
    ) -> Result<(), std::io::Error> {
        debug!(
            "Sending {:x?} len:{} chunk:{}",
            packet,
            packet.serialized_len(),
            self.max_packet_len
        );
        let mut buf: Vec<u8> = Vec::new();
        buf.resize_with(packet.serialized_len() as usize, Default::default);
        let buf = packet.serialize_buffer(&mut buf).unwrap().to_vec();
        for chunk in buf
            .chunks(self.max_packet_len as usize)
            .collect::<Vec<&[u8]>>()
            .iter()
        {
            let _i = self.socket.write(chunk)?;
        }
        Ok(())
    }

    pub fn send_sync(&mut self) -> Result<(), std::io::Error> {
        self.send_packet(mailbox::HidIoPacketBuffer {
            ptype: HidIoPacketType::Sync,
            done: true, // Ready
            ..Default::default()
        })
    }
}

/// A R/W channel for a single endpoint
///
/// This provides an easy interface for other parts of the program to send/recv.
/// messages from without having to worry about the underlying device type.
/// It is responsible for managing the underlying acks/nacks, etc.
/// Process must be continually called.
pub struct HidIoController {
    mailbox: mailbox::Mailbox,
    uid: u64,
    device: HidIoEndpoint,
    received: mailbox::HidIoPacketBuffer,
    receiver: broadcast::Receiver<mailbox::Message>,
    last_sync: Instant,
}

impl HidIoController {
    pub fn new(mailbox: mailbox::Mailbox, uid: u64, device: HidIoEndpoint) -> HidIoController {
        let received = device.create_buffer();
        // Setup receiver so that it can queue up messages between processing loops
        let receiver = mailbox.sender.subscribe();
        let last_sync = Instant::now();
        HidIoController {
            mailbox,
            uid,
            device,
            received,
            receiver,
            last_sync,
        }
    }

    pub fn process(&mut self) -> Result<usize, std::io::Error> {
        let mut io_events = 0;
        match self.device.recv_chunk(&mut self.received) {
            Ok(recv) => {
                if recv > 0 {
                    io_events += 1;
                    self.last_sync = Instant::now();

                    // Handle sync packets
                    if let HidIoPacketType::Sync = &self.received.ptype {
                        self.received = self.device.create_buffer();
                    }
                }
            }
            Err(e) => {
                return Err(e);
            }
        };

        if self.received.done {
            // Send message to mailbox
            let src = mailbox::Address::DeviceHidio { uid: self.uid };
            let dst = mailbox::Address::All;
            let msg = mailbox::Message::new(src, dst, self.received.clone());
            self.mailbox.sender.send(msg).unwrap();
            self.received = self.device.create_buffer();
        }

        if self.last_sync.elapsed().as_secs() >= 5 {
            io_events += 1;
            if self.device.send_sync().is_err() {
                return Err(std::io::Error::new(std::io::ErrorKind::BrokenPipe, ""));
            };
            self.received = self.device.create_buffer();
            self.last_sync = Instant::now();
            return Ok(io_events);
        }

        loop {
            match self.receiver.try_recv() {
                Ok(mut msg) => {
                    // Only look at packets addressed to this endpoint
                    if msg.dst == (mailbox::Address::DeviceHidio { uid: self.uid }) {
                        msg.data.max_len = self.device.max_packet_len;
                        self.device.send_packet(msg.data.clone())?;

                        if msg.data.ptype == HidIoPacketType::Sync {
                            self.received = self.device.create_buffer();
                        }
                    }
                }
                Err(broadcast::error::TryRecvError::Empty) => {
                    break;
                }
                Err(broadcast::error::TryRecvError::Lagged(_skipped)) => {} // TODO (HaaTa): Should probably warn if lagging
                Err(broadcast::error::TryRecvError::Closed) => {
                    return Err(std::io::Error::new(std::io::ErrorKind::BrokenPipe, ""));
                }
            }
        }
        Ok(io_events)
    }
}

/// Supported Ids by this module
/// recursive option applies supported ids from child modules as well
#[allow(unused_variables)]
#[cfg(target_os = "linux")]
pub fn supported_ids(recursive: bool) -> Vec<HidIoCommandId> {
    #[cfg(feature = "dev-capture")]
    {
        let mut ids: Vec<HidIoCommandId> = vec![];
        if recursive {
            ids.extend(evdev::supported_ids().iter().cloned());
        }
        ids
    }

    #[cfg(not(feature = "dev-capture"))]
    vec![]
}

#[cfg(target_os = "macos")]
pub fn supported_ids(_recursive: bool) -> Vec<HidIoCommandId> {
    vec![]
}

#[cfg(target_os = "windows")]
pub fn supported_ids(_recursive: bool) -> Vec<HidIoCommandId> {
    vec![]
}

/// Module initialization
///
/// # Remarks
///
/// Sets up at least one thread per Device (using tokio).
/// Each device is repsonsible for accepting and responding to packet requests.
/// It is also possible to send requests asynchronously back to any Modules.
/// Each device may have it's own RPC API.
#[allow(unused_variables)]
pub async fn initialize(mailbox: mailbox::Mailbox) {
    info!("Initializing devices...");

    #[cfg(all(target_os = "linux", feature = "hidapi-devices"))]
    tokio::join!(
        // Initialize hidapi watcher
        hidapi::initialize(mailbox.clone()),
        // Initialize evdev watcher
        evdev::initialize(mailbox.clone()),
    );

    // Initialize hidapi watcher
    #[cfg(all(target_os = "macos", feature = "hidapi-devices"))]
    hidapi::initialize(mailbox.clone()).await;

    // Initialize hidapi watcher
    #[cfg(all(target_os = "windows", feature = "hidapi-devices"))]
    hidapi::initialize(mailbox.clone()).await;
}

#[cfg(not(feature = "dev-capture"))]
mod evdev {
    use crate::mailbox;

    #[allow(dead_code)]
    pub async fn initialize(_mailbox: mailbox::Mailbox) {}
}
