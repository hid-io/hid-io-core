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

pub mod debug;

/// Handles hidapi devices (libusb/rawhid)
///
/// May also work with bluetooth low energy in the future.
pub mod hidusb;

use crate::api::Endpoint;
use crate::protocol::hidio::*;
use std::collections::HashMap;
use std::sync::mpsc;
use std::sync::mpsc::channel;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use std::io::{Read, Write};

/// A duplex stream for HIDIO to communicate over
pub trait HIDIOTransport: Read + Write {}

const MAX_RECV_SIZE: usize = 1024;

/// A raw transport plus any associated metadata
///
/// Contains helpers to encode/decode HIDIO packets
pub struct HIDIOEndpoint {
    socket: Box<HIDIOTransport>,
    max_packet_len: u32,
}

impl HIDIOEndpoint {
    pub fn new(socket: Box<HIDIOTransport>, max_packet_len: u32) -> HIDIOEndpoint {
        HIDIOEndpoint {
            socket,
            max_packet_len,
        }
    }

    pub fn recv_chunk(&mut self, buffer: &mut HIDIOPacketBuffer) -> Result<usize, std::io::Error> {
        use std::io::Read;
        let mut rbuf = [0; MAX_RECV_SIZE];
        match self.socket.read(&mut rbuf) {
            Ok(len) => {
                if len > 0 {
                    let slice = &rbuf[0..len];
                    let ret = buffer.decode_packet(&mut slice.to_vec());
                    if let Err(e) = ret {
                        error!("recv_chunk({}) {:?}", len, e);
                        println!("received: {:?}", slice);
                        println!("current state: {:?}", buffer);
                        std::process::exit(2);
                    } else {
                        info!("R{} {:x?}", buffer.data.len(), buffer);
                    }
                }

                Ok(len)
            }
            Err(e) => Err(e),
        }
    }

    pub fn create_buffer(&self) -> HIDIOPacketBuffer {
        let mut buffer = HIDIOPacketBuffer::new();
        buffer.max_len = self.max_packet_len;
        buffer
    }

    pub fn recv_packet(&mut self) -> HIDIOPacketBuffer {
        let mut deserialized = self.create_buffer();

        while !deserialized.done {
            if let Ok(len) = self.recv_chunk(&mut deserialized) {
                if len > 0 {
                    match &deserialized.ptype {
                        HIDIOPacketType::Sync => {
                            deserialized = self.create_buffer();
                        }
                        HIDIOPacketType::ACK => {
                            // Don't ack an ack
                        }
                        HIDIOPacketType::NAK => {
                            println!("NACK");
                            break;
                        }
                        HIDIOPacketType::Continued | HIDIOPacketType::Data => {
                            self.send_ack(deserialized.id, vec![]);
                        }
                    }
                }
            }
        }

        //info!("Received {:x?}", deserialized);
        deserialized
    }

    pub fn send_packet(&mut self, mut packet: HIDIOPacketBuffer) -> Result<(), std::io::Error> {
        use std::io::Write;
        info!("Sending {:x?}", packet);
        let buf: Vec<u8> = packet.serialize_buffer().unwrap();
        for chunk in buf
            .chunks(self.max_packet_len as usize)
            .collect::<Vec<&[u8]>>()
            .iter()
        {
            self.socket.write(chunk)?;
        }
        Ok(())
    }

    pub fn send_sync(&mut self) {
        self.send_packet(HIDIOPacketBuffer {
            ptype: HIDIOPacketType::Sync,
            id: 0,
            max_len: 64, //..Defaults
            data: vec![],
            done: true,
        })
        .unwrap();
    }

    pub fn send_ack(&mut self, _id: u32, data: Vec<u8>) {
        self.send_packet(HIDIOPacketBuffer {
            ptype: HIDIOPacketType::ACK,
            id: 0,       // id,
            max_len: 64, //..Defaults
            data,
            done: true,
        })
        .unwrap();
    }
}

/// A R/W channel for a single endpoint
///
/// This provides an easy interface for other parts of the program to send/recv.
/// messages from without having to worry about the underlying device type.
/// It is responsible for managing the underlying acks/nacks, etc.
/// Process must be continually called.
pub struct HIDIOController {
    id: String,
    device: HIDIOEndpoint,
    received: HIDIOPacketBuffer,
    last_sync: Instant,
    message_queue: std::sync::mpsc::Sender<HIDIOPacketBuffer>,
    response_queue: std::sync::mpsc::Receiver<HIDIOPacketBuffer>,
}

impl HIDIOController {
    pub fn new(
        id: String,
        device: HIDIOEndpoint,
        message_queue: std::sync::mpsc::Sender<HIDIOPacketBuffer>,
        response_queue: std::sync::mpsc::Receiver<HIDIOPacketBuffer>,
    ) -> HIDIOController {
        let received = device.create_buffer();
        let last_sync = Instant::now();
        //let mut prev_len = 0;
        HIDIOController {
            device,
            id,
            received,
            last_sync,
            message_queue,
            response_queue,
        }
    }

    pub fn process(&mut self) -> Result<(), std::io::Error> {
        match self.device.recv_chunk(&mut self.received) {
            Ok(recv) => {
                if recv > 0 {
                    self.last_sync = Instant::now();

                    /*let len = received.data.len();
                    //println!("[{}..{}]", prev_len, len);
                    info!("<{:?}>", &received.data[prev_len..len].iter().map(|x| *x as char).collect::<Vec<char>>());
                    prev_len = received.data.len();*/

                    match &self.received.ptype {
                        HIDIOPacketType::Sync => {
                            self.received = self.device.create_buffer();
                        }
                        HIDIOPacketType::ACK => {
                            // Don't ack an ack
                        }
                        HIDIOPacketType::NAK => {
                            println!("NACK. Resetting buffer");
                            self.received = self.device.create_buffer();
                        }
                        HIDIOPacketType::Continued | HIDIOPacketType::Data => {}
                    }

                    if !self.received.done {
                        self.device.send_ack(self.received.id, vec![]);
                    }
                }
            }
            Err(e) => {
                return Err(e);
                //::std::process::exit(1);
            }
        };

        if self.received.done {
            self.message_queue.send(self.received.clone()).unwrap();
            self.received = self.device.create_buffer();
            //prev_len = 0;
        }

        if self.last_sync.elapsed().as_secs() >= 5 {
            self.device.send_sync();
            self.received = self.device.create_buffer();
            self.last_sync = Instant::now();
            return Ok(());
        }

        match self.response_queue.try_recv() {
            Ok(mut p) => {
                p.max_len = self.device.max_packet_len;
                self.device.send_packet(p.clone())?;

                if p.ptype == HIDIOPacketType::Sync {
                    self.received = self.device.create_buffer();
                }
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                return Err(std::io::Error::new(std::io::ErrorKind::BrokenPipe, ""));
                //::std::process::exit(1);
            }
        }

        return Ok(());
    }
}

/// The userspace end of a HIDIOController
///
/// Can be cloned and passed around freely, even between threads
pub struct HIDIOQueue {
    pub info: Endpoint,
    message_queue: std::sync::mpsc::Receiver<HIDIOPacketBuffer>,
    response_queue: std::sync::mpsc::Sender<HIDIOPacketBuffer>,
}

impl HIDIOQueue {
    pub fn new(
        info: Endpoint,
        message_queue: std::sync::mpsc::Receiver<HIDIOPacketBuffer>,
        response_queue: std::sync::mpsc::Sender<HIDIOPacketBuffer>,
    ) -> HIDIOQueue {
        HIDIOQueue {
            info,
            message_queue,
            response_queue,
        }
    }

    pub fn send_packet(
        &self,
        packet: HIDIOPacketBuffer,
    ) -> Result<(), mpsc::SendError<HIDIOPacketBuffer>> {
        self.response_queue.send(packet)
    }

    pub fn recv_packet(&mut self) -> HIDIOPacketBuffer {
        self.message_queue.recv().unwrap()
    }

    pub fn messages(&mut self) -> mpsc::TryIter<HIDIOPacketBuffer> {
        // TODO: Detect error (other side disconnected)
        self.message_queue.try_iter()
    }
}

/// A single message and its recipient or source
#[derive(Debug, Clone)]
pub struct HIDIOMessage {
    pub device: String,
    pub message: HIDIOPacketBuffer,
}

/// The main HIDIOMessage passer
///
/// Will grab messages from every "mailbox" and pass the message to the correct outgoing queue.
/// Will monitor each incoming queue and leave a copy of the message in every mailbox.
/// Process must be continually called.
pub struct HIDIOMailer {
    devices: HashMap<String, HIDIOQueue>,
    connected: Arc<RwLock<Vec<Endpoint>>>,
    incoming: std::sync::mpsc::Receiver<HIDIOMessage>,
    outgoing: Vec<std::sync::mpsc::Sender<HIDIOMessage>>,
}

impl HIDIOMailer {
    pub fn new(incoming: std::sync::mpsc::Receiver<HIDIOMessage>) -> HIDIOMailer {
        let devices = HashMap::new();
        let outgoing = vec![];
        let connected = Arc::new(RwLock::new(vec![]));

        HIDIOMailer {
            devices,
            connected,
            incoming,
            outgoing,
        }
    }

    pub fn register_device(&mut self, id: String, device: HIDIOQueue) {
        info!("Registering device: {}", id);
        let mut connected = self.connected.write().unwrap();
        (*connected).push(device.info.clone());
        self.devices.insert(id, device);
    }

    pub fn unregister_device(&mut self, id: &str) {
        info!("Unregistering device: {}", id);
        let mut connected = self.connected.write().unwrap();
        *connected = connected
            .drain_filter(|dev| dev.id.to_string() != id)
            .collect::<Vec<_>>();
        self.devices.remove(id);
    }

    pub fn devices(&self) -> Arc<RwLock<Vec<Endpoint>>> {
        self.connected.clone()
    }

    pub fn register_listener(&mut self, sink: std::sync::mpsc::Sender<HIDIOMessage>) {
        self.outgoing.push(sink);
    }

    pub fn process(&mut self) {
        for (device, queue) in self.devices.iter_mut() {
            for message in queue.messages() {
                let m = HIDIOMessage {
                    device: device.to_string(),
                    message,
                };
                for sink in self.outgoing.iter() {
                    sink.send(m.clone()).unwrap();
                }
            }
        }

        for message in self.incoming.try_iter() {
            let device = &self.devices[&message.device];
            let ret = device.send_packet(message.message);
            if ret.is_err() {
                info!("Device queue disconnected. Unregistering.");
                self.devices.remove(&message.device);
            }
        }
    }
}

/// The userspace end of the HIDIO message system
///
/// Can receive all incoming messages, or send a new message to a device by id.
/// Provides utility functions to construct common messages.
pub struct HIDIOMailbox {
    pub nodes: Arc<RwLock<Vec<Endpoint>>>,
    incoming: std::sync::mpsc::Receiver<HIDIOMessage>,
    outgoing: std::sync::mpsc::Sender<HIDIOMessage>,
}

impl HIDIOMailbox {
    pub fn new(
        nodes: Arc<RwLock<Vec<Endpoint>>>,
        incoming: std::sync::mpsc::Receiver<HIDIOMessage>,
        outgoing: std::sync::mpsc::Sender<HIDIOMessage>,
    ) -> HIDIOMailbox {
        HIDIOMailbox {
            nodes,
            incoming,
            outgoing,
        }
    }

    pub fn from_sender(
        dest: mpsc::Sender<HIDIOMessage>,
        nodes: Arc<RwLock<Vec<Endpoint>>>,
    ) -> (mpsc::Sender<HIDIOMessage>, HIDIOMailbox) {
        let (writer, reader) = channel::<HIDIOMessage>();
        let mailbox = HIDIOMailbox::new(nodes, reader, dest);
        (writer, mailbox)
    }

    pub fn send_packet(&self, device: String, packet: HIDIOPacketBuffer) {
        let result = self.outgoing.send(HIDIOMessage {
            device,
            message: packet,
        });
        if let Err(e) = result {
            error!("send_packet {}", e);
        }
    }

    pub fn recv(&self) -> HIDIOMessage {
        self.incoming.recv().unwrap()
    }

    pub fn recv_psuedoblocking(&self) -> Option<HIDIOMessage> {
        match self.incoming.recv_timeout(Duration::from_millis(1)) {
            Ok(m) => Some(m),
            Err(mpsc::RecvTimeoutError::Timeout) => None,
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                warn!("Lost socket"); // TODO: pass warning down
                std::process::exit(1);
            }
        }
    }

    pub fn iter(&self) -> mpsc::Iter<HIDIOMessage> {
        self.incoming.iter()
    }

    pub fn send_sync(&self, device: String) {
        self.send_packet(
            device,
            HIDIOPacketBuffer {
                ptype: HIDIOPacketType::Sync,
                id: 0,
                max_len: 64, //..Defaults
                data: vec![],
                done: true,
            },
        );
    }

    pub fn send_ack(&self, device: String, _id: u32, data: Vec<u8>) {
        self.send_packet(
            device,
            HIDIOPacketBuffer {
                ptype: HIDIOPacketType::ACK,
                id: 0,       // id,
                max_len: 64, //..Defaults
                data,
                done: true,
            },
        );
    }

    pub fn send_nack(&self, device: String, id: u32, data: Vec<u8>) {
        self.send_packet(
            device,
            HIDIOPacketBuffer {
                ptype: HIDIOPacketType::NAK,
                id,
                max_len: 64, //..Defaults
                data,
                done: true,
            },
        );
    }

    pub fn send_command(&self, device: String, id: HIDIOCommandID, data: Vec<u8>) {
        self.send_packet(
            device,
            HIDIOPacketBuffer {
                ptype: HIDIOPacketType::Data,
                id: id as u32,
                max_len: 64, //..Defaults
                data,
                done: true,
            },
        );
    }
}

/// Module initialization
///
/// # Remarks
///
/// Sets up at least one thread per Device.
/// Each device is repsonsible for accepting and responding to packet requests.
/// It is also possible to send requests asynchronously back to any Modules.
/// Each device may have it's own RPC API.
pub fn initialize(mailer: HIDIOMailer) {
    info!("Initializing devices...");

    // Initialize device watcher threads
    hidusb::initialize(mailer);

    debug::initialize();
}
