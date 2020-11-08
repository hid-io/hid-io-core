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

/// Mailbox
/// Handles message passing between devices, modules and api calls
/// Uses a broadcast channel to handle communication
// ----- Modules -----
use crate::api::Endpoint;
use crate::protocol::hidio;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::stream::StreamExt;
use tokio::sync::broadcast;

// ----- Enumerations -----

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Address {
    // All/any addressed (used as a broadcast destination, not as a source)
    All,
    // Capnproto API address, with node uid
    ApiCapnp {
        uid: u64,
    },
    // Cancel all subscriptions
    CancelAllSubscriptions,
    // Cancel subscription
    // Used to gracefully end message streams
    CancelSubscription {
        // Uid of endpoint of the subscription
        uid: u64,
        // Subscription id
        sid: u64,
    },
    // HidIo address, with node uid
    DeviceHidio {
        uid: u64,
    },
    // Generic HID address, with nod uid
    DeviceHid {
        uid: u64,
    },
    // Drop subscription
    DropSubscription,
    // Module address
    Module,
}

// ----- Consts -----

/// Number of message slots for the mailbox broadcast channel
/// Must be equal to the largest queue needed for the slowest receiver
const CHANNEL_SLOTS: usize = 100;

// ----- Structs -----

/// HID-IO Mailbox
///
/// Handles passing messages to various components inside of HID-IO
/// Best thought of as a broadcast style packet switcher.
/// Each thread (usually async tokio) is given a receiver and can filter for
/// any desired packets.
/// This is not quite as effecient as direct channels; however, this greatly
/// simplifies message passing across HID-IO. Making it easier to add new modules.
///
/// This struct can be safely cloned and passed around anywhere in the codebase.
/// In most cases only the sender field is used (as it has the subscribe() function as well).
#[derive(Clone, Debug)]
pub struct Mailbox {
    pub nodes: Arc<RwLock<Vec<Endpoint>>>,
    pub last_uid: Arc<RwLock<u64>>,
    pub lookup: Arc<RwLock<HashMap<String, Vec<u64>>>>,
    pub sender: broadcast::Sender<Message>,
    pub ack_timeout: Arc<RwLock<std::time::Duration>>,
}

impl Mailbox {
    pub fn new() -> Mailbox {
        // Create broadcast channel
        let (sender, _) = broadcast::channel::<Message>(CHANNEL_SLOTS);
        // Setup nodes list
        let nodes = Arc::new(RwLock::new(vec![]));
        // Setup nodes lookup table
        let lookup = Arc::new(RwLock::new(HashMap::new()));
        // Setup last uid assigned (uids are reused when possible for devices)
        let last_uid: Arc<RwLock<u64>> = Arc::new(RwLock::new(0));
        // Setup default timeout of 2 seconds
        let ack_timeout: Arc<RwLock<std::time::Duration>> =
            Arc::new(RwLock::new(std::time::Duration::from_millis(2000)));
        Mailbox {
            nodes,
            last_uid,
            lookup,
            sender,
            ack_timeout,
        }
    }

    /// Attempt to locate an unused id for the device key
    pub fn get_uid(&mut self, key: String, path: String) -> Option<u64> {
        let mut lookup = self.lookup.write().unwrap();
        let lookup_entry = lookup.entry(key).or_default();

        // Locate an id
        'outer: for uid in lookup_entry.iter() {
            for mut node in (*self.nodes.read().unwrap()).clone() {
                if node.uid() == *uid {
                    // Id is being used, and has the same path (i.e. this device)
                    if node.path() == path {
                        // Return an invalid Id (0)
                        return Some(0);
                    }

                    // Id is being used, and is not available
                    continue 'outer;
                }
            }
            // Id is not being used
            return Some(*uid);
        }

        // Could not locate an id
        None
    }

    /// Add uid to lookup
    pub fn add_uid(&mut self, key: String, uid: u64) {
        let mut lookup = self.lookup.write().unwrap();
        let lookup_entry = lookup.entry(key).or_default();
        lookup_entry.push(uid);
    }

    /// Assign uid
    /// This function will attempt to lookup an existing id first
    /// And generate a new uid if necessary
    /// An error is returned if this lookup already has a uid (string+path)
    pub fn assign_uid(&mut self, key: String, path: String) -> Result<u64, std::io::Error> {
        match self.get_uid(key.clone(), path) {
            Some(0) => Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "uid has already been registered!",
            )),
            Some(uid) => Ok(uid),
            None => {
                // Get last created id and increment
                (*self.last_uid.write().unwrap()) += 1;
                let uid = *self.last_uid.read().unwrap();

                // Add id to lookup
                self.add_uid(key, uid);
                Ok(uid)
            }
        }
    }

    /// Register node as an endpoint (device or api)
    pub fn register_node(&mut self, mut endpoint: Endpoint) {
        info!("Registering endpoint: {}", endpoint.uid());
        let mut nodes = self.nodes.write().unwrap();
        (*nodes).push(endpoint);
    }

    /// Unregister node as an endpoint (device or api)
    pub fn unregister_node(&mut self, uid: u64) {
        info!("Unregistering endpoint: {}", uid);
        let mut nodes = self.nodes.write().unwrap();
        *nodes = nodes
            .drain_filter(|dev| dev.uid() != uid)
            .collect::<Vec<_>>();
    }

    /// Convenience function to send a HidIo Command to device using the mailbox
    /// Returns the ACK message if enabled.
    /// ACK will timeout if it exceeds self.ack_timeout
    pub async fn send_command(
        &self,
        src: Address,
        dst: Address,
        id: hidio::HidIoCommandID,
        data: Vec<u8>,
        ack: bool,
    ) -> Result<Option<Message>, AckWaitError> {
        // Select packet type
        /* TODO Add firmware support for NAData
        let ptype = if ack {
            hidio::HidIoPacketType::Data
        } else {
            hidio::HidIoPacketType::NAData
        };
        */
        let ptype = hidio::HidIoPacketType::Data;

        // Construct command packet
        let data = hidio::HidIoPacketBuffer {
            ptype,
            id,
            max_len: 64, //..Defaults
            data,
            done: true,
        };

        // Check receiver count
        if self.sender.receiver_count() == 0 {
            error!("send_command (no active receivers)");
            return Err(AckWaitError::NoActiveReceivers);
        }

        // Subscribe to messages before sending message, but this means we have to check the
        // receiver count earlier
        let receiver = self.sender.subscribe();

        // Construct command message and broadcast
        let result = self.sender.send(Message {
            src,
            dst,
            data: data.clone(),
        });

        if let Err(e) = result {
            error!(
                "send_command failed, something is odd, should not get here... {:?}",
                e
            );
            return Err(AckWaitError::NoActiveReceivers);
        }

        // No ACK data packet command, no ACK to wait for
        if !ack {
            return Ok(None);
        }

        // Construct stream filter
        tokio::pin! {
            let stream = receiver.into_stream()
                .filter(Result::is_ok)
                .map(Result::unwrap)
                .filter(|msg| msg.src == src && msg.dst == dst && msg.data.id == id);
        }

        // Wait on filtered messages
        let ack_timeout = *self.ack_timeout.read().unwrap();
        loop {
            match tokio::time::timeout(ack_timeout, stream.next()).await {
                Ok(msg) => {
                    if let Some(msg) = msg {
                        match msg.data.ptype {
                            hidio::HidIoPacketType::ACK => {
                                return Ok(Some(msg));
                            }
                            // We may still want the message data from a NAK
                            hidio::HidIoPacketType::NAK => {
                                return Err(AckWaitError::NAKReceived { msg });
                            }
                            _ => {}
                        }
                    } else {
                        return Err(AckWaitError::Invalid);
                    }
                }
                Err(_) => {
                    warn!(
                        "Timeout ({:?}) receiving ACK for: {}",
                        ack_timeout,
                        data
                    );
                    return Err(AckWaitError::Timeout);
                }
            }
        }
    }

    /// Convenience function to send a HidIo Command to device using the mailbox
    /// Returns the ACK message if enabled.
    /// This is the blocking version of send_command().
    /// ACK will timeout if it exceeds self.ack_timeout
    pub fn try_send_command(
        &self,
        src: Address,
        dst: Address,
        id: hidio::HidIoCommandID,
        data: Vec<u8>,
        ack: bool,
    ) -> Result<Option<Message>, AckWaitError> {
        // Select packet type
        /* TODO Add firmware support for NAData
        let ptype = if ack {
            hidio::HidIoPacketType::Data
        } else {
            hidio::HidIoPacketType::NAData
        };
        */
        let ptype = hidio::HidIoPacketType::Data;

        // Construct command packet
        let data = hidio::HidIoPacketBuffer {
            ptype,
            id,
            max_len: 64, //..Defaults
            data,
            done: true,
        };

        // Check receiver count
        if self.sender.receiver_count() == 0 {
            error!("send_command (no active receivers)");
            return Err(AckWaitError::NoActiveReceivers);
        }

        // Subscribe to messages before sending message, but this means we have to check the
        // receiver count earlier
        let mut receiver = self.sender.subscribe();

        // Construct command message and broadcast
        let result = self.sender.send(Message { src, dst, data });

        if let Err(e) = result {
            error!(
                "send_command failed, something is odd, should not get here... {:?}",
                e
            );
            return Err(AckWaitError::NoActiveReceivers);
        }

        // No ACK data packet command, no ACK to wait for
        if !ack {
            return Ok(None);
        }

        // Loop until we find the message we want
        let start_time = std::time::Instant::now();
        loop {
            // Check for timeout
            if start_time.elapsed() >= *self.ack_timeout.read().unwrap() {
                warn!(
                    "Timeout ({:?}) receiving ACK for command: src:{:?} dst:{:?}",
                    *self.ack_timeout.read().unwrap(),
                    src,
                    dst
                );
                return Err(AckWaitError::Timeout);
            }

            // Attempt to receive message
            match receiver.try_recv() {
                Ok(msg) => {
                    // Packet must have the same address as was sent, except reversed
                    if msg.dst == src && msg.src == dst && msg.data.id == id {
                        match msg.data.ptype {
                            hidio::HidIoPacketType::ACK => {
                                return Ok(Some(msg));
                            }
                            // We may still want the message data from a NAK
                            hidio::HidIoPacketType::NAK => {
                                return Err(AckWaitError::NAKReceived { msg });
                            }
                            _ => {}
                        }
                    }
                }
                Err(broadcast::error::TryRecvError::Empty) => {
                    // Sleep while queue is empty
                    std::thread::yield_now();
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
                Err(broadcast::error::TryRecvError::Lagged(_skipped)) => {} // TODO (HaaTa): Should probably warn if lagging
                Err(broadcast::error::TryRecvError::Closed) => {
                    // Channel has closed, this is very bad
                    return Err(AckWaitError::ChannelClosed);
                }
            }
        }
    }

    pub fn drop_subscriber(&self, uid: u64, sid: u64) {
        // Construct a dummy message
        let data = hidio::HidIoPacketBuffer::default();

        // Construct command message and broadcast
        let result = self.sender.send(Message {
            src: Address::DropSubscription,
            dst: Address::CancelSubscription { uid, sid },
            data,
        });

        if let Err(e) = result {
            error!("drop_subscriber {:?}", e);
        }
    }

    pub fn drop_all_subscribers(&self) {
        // Construct a dummy message
        let data = hidio::HidIoPacketBuffer::default();

        // Construct command message and broadcast
        let result = self.sender.send(Message {
            src: Address::DropSubscription,
            dst: Address::CancelAllSubscriptions,
            data,
        });

        if let Err(e) = result {
            error!("drop_all_subscribers {:?}", e);
        }
    }
}

impl Default for Mailbox {
    fn default() -> Self {
        Self::new()
    }
}

/// Container for HidIoPacketBuffer
/// Used to indicate the source and destinations inside of hid-io-core.
/// Also contains a variety of convenience functions using the src and dst information.
#[derive(PartialEq, Clone, Debug)]
pub struct Message {
    pub src: Address,
    pub dst: Address,
    pub data: hidio::HidIoPacketBuffer,
}

impl Message {
    pub fn new(src: Address, dst: Address, data: hidio::HidIoPacketBuffer) -> Message {
        Message { src, dst, data }
    }

    /// Acknowledgement of a HidIo packet
    pub fn send_ack(&self, sender: broadcast::Sender<Message>, data: Vec<u8>) {
        let src = self.dst;
        let dst = self.src;

        // Construct ack packet
        let data = hidio::HidIoPacketBuffer {
            ptype: hidio::HidIoPacketType::ACK,
            id: self.data.id, // id,
            max_len: 64,      // Default
            data,
            done: true,
        };

        // Construct ack message and broadcast
        let result = sender.send(Message { src, dst, data });

        if let Err(e) = result {
            error!("send_ack {:?}", e);
        }
    }

    /// Rejection/NAK of a HidIo packet
    pub fn send_nak(&self, sender: broadcast::Sender<Message>, data: Vec<u8>) {
        let src = self.dst;
        let dst = self.src;

        // Construct ack packet
        let data = hidio::HidIoPacketBuffer {
            ptype: hidio::HidIoPacketType::NAK,
            id: self.data.id, // id,
            max_len: 64,      // Default
            data,
            done: true,
        };

        // Construct ack message and broadcast
        let result = sender.send(Message { src, dst, data });

        if let Err(e) = result {
            error!("send_ack {:?}", e);
        }
    }
}

#[derive(Debug)]
pub enum AckWaitError {
    TooManySyncs,
    NAKReceived { msg: Message },
    Invalid,
    NoActiveReceivers,
    Timeout,
    ChannelClosed,
}
