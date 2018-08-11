/* Copyright (C) 2017-2018 by Jacob Alexander
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

// ----- Crates -----

extern crate bincode;
extern crate serde;



// ----- Modules -----

use std::fmt;

use self::bincode::serialize;
use self::serde::ser::{self, Serialize, Serializer, SerializeSeq};



// ----- Enumerations -----

/// HID-IO Packet Types
///
/// # Remarks
/// Must not be larger than 0x7, 5-7 are reserved.
#[derive(PartialEq, Clone, Copy)]
pub enum HIDIOPacketType {
    /// Data packet
    Data      = 0,
    /// ACK packet
    ACK       = 1,
    /// NAK packet
    NAK       = 2,
    /// Sync packet
    Sync      = 3,
    /// Continued packet
    Continued = 4,
}



// ----- Structs -----

/// HID-IO Packet Buffer Struct
///
/// # Remarks
/// Used to store HID-IO data chunks. Will be chunked into individual packets on transmission.
#[derive(PartialEq, Clone)]
pub struct HIDIOPacketBuffer {
    /// Type of packet (Continued is automatically set if needed)
    pub ptype:   HIDIOPacketType,
    /// Packet Id
    pub id:      u32,
    /// Packet length for serialization (in bytes)
    pub max_len: u32,
    /// Payload data, chunking is done automatically by serializer
    pub data:    Vec<u8>,
    /// Set False if buffer is not complete, True if it is
    pub done:    bool,
}

/// HID-IO Parse Error
///
/// # Remarks thrown when there's an issue processing byte stream.
pub struct HIDIOParseError {
}



// ----- Utility Functions -----

/// Determines the packet type from a byte stream
///
/// # Arguments
/// * `packet_data` - Vector of bytes
///
/// # Remarks
/// Uses a packet byte stream to determine the packet type.
/// First three bits of data stream are used (from C-Struct):
///
/// ```
/// struct HIDIO_Packet {
///    HIDIO_Packet_Type type:3;
///    ...
/// };
/// ```
pub fn packet_type(packet_data: &mut Vec<u8>) -> Result<HIDIOPacketType, HIDIOParseError> {
    let packet_data_len = packet_data.len();

    // Check if the byte stream is large enough
    if packet_data_len < 1 {
        return Err(HIDIOParseError{});
    }

    // Extract first 3 bits from first byte
    let ptype: u8 = (packet_data[0] & 0xE0) >> 5;

    // Convert to HIDIOPacketType enum
    match ptype {
        0 => Ok(HIDIOPacketType::Data),
        1 => Ok(HIDIOPacketType::ACK),
        2 => Ok(HIDIOPacketType::NAK),
        3 => Ok(HIDIOPacketType::Sync),
        4 => Ok(HIDIOPacketType::Continued),
        _ => Err(HIDIOParseError{}),
    }
}

/// Determines payload of packet from a byte stream
///
/// # Arguments
/// * `packet_data` - Vector of bytes
///
/// # Remarks
/// Uses a packet byte stream to determine payload length.
/// This length does not include the first 2 packet bytes in the overall packet length.
/// The length does include the bytes used for the packet Id.
///
/// ```
/// struct HIDIO_Packet {
///    ... (6 bits)
///    uint8_t           upper_len:2; // Upper 2 bits of length field (generally unused)
///    uint8_t           len;         // Lower 8 bits of length field
///    ...
/// };
pub fn payload_len(packet_data: &mut Vec<u8>) -> Result<u32, HIDIOParseError> {
    let packet_data_len = packet_data.len();

    // Check if the byte stream is large enough
    if packet_data_len < 2 {
        return Err(HIDIOParseError{});
    }

    // Extract upper_len and len
    let upper_len = (packet_data[0] & 0x3) as u32;
    let len = packet_data[1] as u32;

    // Merge
    let payload_len: u32 = upper_len << 8 | len;

    Ok(payload_len)
}

/// Determines id_width from a byte stream
///
/// # Arguments
/// * `packet_data` - Vector of bytes
///
/// # Remarks
/// Uses a packet byte stream to determine packet id_width.
///
/// ```
/// struct HIDIO_Packet {
///    ... (4 bits)
///    uint8_t           id_width:1;  // 0 - 16bits, 1 - 32bits
///    ...
/// };
pub fn packet_id_width(packet_data: &mut Vec<u8>) -> Result<usize, HIDIOParseError> {
    let packet_data_len = packet_data.len();

    // Check if the byte stream is large enough
    if packet_data_len < 2 {
        return Err(HIDIOParseError{});
    }

    // Extract id_width
    match packet_data[0] & 0x08 {
        0x00 => Ok(2), // 16 bit
        0x08 => Ok(4), // 32 bit
        _ =>    Err(HIDIOParseError{}),
    }
}

/// Determines packet id from a byte stream
///
/// # Arguments
/// * `packet_data` - Vector of bytes
///
/// # Remarks
/// Uses a packet byte stream to determine packet Id.
///
/// ```
/// struct HIDIO_Packet {
///    ... (4 bits)
///    uint8_t           id_width:1;  // 0 - 16bits, 1 - 32bits
///    ... (11 bits)
///    uint16_t/uint32_t id;          // Id field (check id_width to see which struct to use)
///    ...
/// };
pub fn packet_id(packet_data: &mut Vec<u8>) -> Result<u32, HIDIOParseError> {
    let packet_data_len = packet_data.len();

    // Extract id_width
    let id_width = packet_id_width(packet_data)?;

    // Make sure there are enough possible bytes
    if payload_len(packet_data)? < id_width as u32 {
        return Err(HIDIOParseError{});
    }

    // Make sure there enough actual bytes
    if packet_data_len < id_width + 2 {
        return Err(HIDIOParseError{});
    }

    // Iterate over bytes, constructing Id of either 16 or 32 bit width
    let mut id: u32 = 0;
    let offset = 2;
    for idx in 0 .. id_width as usize {
        id |= (packet_data[offset + idx] as u32) << (idx * 8);
    }

    Ok(id)
}

/// Determines whether there are following continued packets
///
/// # Arguments
/// * `packet_data` - Vector of bytes
///
/// # Remarks
/// Uses a packet byte stream to determine cont field.
///
/// ```
/// struct HIDIO_Packet {
///    ... (3 bits)
///    uint8_t           cont:1;      // 0 - Only packet, 1 continued packet following
///    ...
/// };
pub fn continued_packet(packet_data: &mut Vec<u8>) -> Result<bool, HIDIOParseError> {
    let packet_data_len = packet_data.len() as u32;

    // Check if the byte stream is large enough
    if packet_data_len < 1 {
        return Err(HIDIOParseError{});
    }

    // Extract cont field
    // Determine value
    match packet_data[0] & 0x10 {
        0x10 => Ok(true),
        0x00 => Ok(false),
        _ =>    Err(HIDIOParseError{}),
    }
}

/// Determines the starting position of the payload data
///
/// # Arguments
/// * `packet_data` - Vector of bytes
///
/// # Remarks
/// Uses a packet byte stream to find payload start.
/// Please note that there may be no payload, or Id.
/// In this case the starting position will be index 2.
pub fn payload_start(packet_data: &mut Vec<u8>) -> Result<usize, HIDIOParseError> {
    // Retrieve id_width
    let id_width = packet_id_width(packet_data)?;

    // Retrieve payload_len, if 0, then return 2 (minimum packet size)
    if payload_len(packet_data)? == 0 {
        return Ok(2);
    }

    // Determine starting position
    Ok(2 + id_width as usize)
}



// ----- Implementations -----

impl HIDIOPacketBuffer {
    /// Constructor for HIDIOPacketBuffer
    ///
    /// # Remarks
    /// Initialize as blank
    pub fn new() -> HIDIOPacketBuffer {
        HIDIOPacketBuffer {
            ptype:   HIDIOPacketType::Data,
            id:      0,
            max_len: 0,
            data:    vec![],
            done:    false,
        }
    }

    /// Append payload data
    ///
    /// # Arguments
    /// * `new_data` - Vector of bytes
    ///
    /// # Remarks
    /// Appends payload to HIDIOPacketBuffer.
    pub fn append_payload(&mut self, new_data: &mut Vec<u8>) -> bool {
        // Check if buffer was already finished
        if !self.done {
            warn!("HIDIOPacketBuffer is already 'done'");
            return false;
        }

        self.data.append(new_data);
        true
    }

    /// Append packet stream
    /// Returns the number of bytes used.
    ///
    /// # Arguments
    /// * `packet_data` - Vector of bytes of packet data
    ///
    /// # Remarks
    /// Does packet decoding on the fly.
    /// Will set done parameter if this is the last packet.
    pub fn decode_packet(&mut self, packet_data: &mut Vec<u8>) -> Result<u32, HIDIOParseError> {
        // Check if buffer was already finished
        if self.done {
            warn!("HIDIOPacketBuffer is already 'done'");
            return Ok(0);
        }

        let packet_data_len = packet_data.len() as u32;

        // Get packet type
        let ptype = packet_type(packet_data)?;

        // Get payload_len
        let payload_len = payload_len(packet_data)?;
        let packet_len = payload_len + 2;

        // Make sure there's actually payload_len available
        if packet_data_len - 2 < payload_len {
            warn!("Dropping. Not enough bytes available in packet stream. got:{}, expected:{}", packet_data_len - 2, payload_len);
            return Ok(packet_data_len);
        }

        // Get packet Id
        let id = packet_id(packet_data)?;

        // Is this a new packet?
        // More information to set, if initializing buffer
        if self.data.len() == 0 && ptype != HIDIOPacketType::Continued {
            // Set packet type
            self.ptype = ptype;

            // Set packet id
            self.id = id;

        // Make sure the current buffer matches what we're expecting
        } else {
            // Check for invalid packet type
            if self.data.len() == 0 && ptype == HIDIOPacketType::Continued {
                warn!("Dropping. Invalid packet type when initializing buffer, HIDIOPacketType::Continued");
                return Ok(packet_len);
            }

            // Check if not a continued packet, and we have a payload
            if self.data.len() > 0 && ptype != HIDIOPacketType::Continued {
                warn!("Dropping. Invalid packet type (non-HIDIOPacketType::Continued) on a already initialized buffer");
                return Ok(packet_len);
            }

            // Validate that we're looking at the same Id
            if self.id != id {
                warn!("Dropping. Invalid incoming id:{}, expected:{}", id, self.id);
                return Ok(packet_len);
            }
        }

        // Payload start
        let payload_start = payload_start(packet_data)?;

        // Get id_width_len
        let id_width_len = packet_id_width(packet_data)?;

        // Check if this buffer will be completed
        self.done = !continued_packet(packet_data)?;

        // Add payload
        let slice = &packet_data[payload_start .. payload_start + payload_len as usize - id_width_len];
        self.data.append(&mut slice.to_vec());

        // Finished
        Ok(packet_len)
    }

    /// Serialize HIDIOPacketBuffer
    ///
    /// # Remarks
    /// Provides a raw data vector to the serialized data.
    /// Removes some of the header that Serialize from serde prepends.
    pub fn serialize_buffer(&mut self) -> Result<Vec<u8>, HIDIOParseError> {
        // Serialize
        let serialized: Vec<u8> = serialize(&self).unwrap();

        // Make sure serialization worked
        if serialized.len() < 10 {
            return Err(HIDIOParseError{});
        }

        // Slice off the first 8 header bytes from serde
        let slice = &serialized[8 .. ];
        let serialized = slice.to_vec();

        Ok(serialized)
    }
}

impl Serialize for HIDIOPacketBuffer {
    /// Serializer for HIDIOPacketBuffer
    ///
    /// # Remarks
    /// Determine cont, width, upper_len and len fields
    /// According to this C-Struct:
    ///
    /// ```
    /// struct HIDIO_Packet {
    ///    HIDIO_Packet_Type type:3;
    ///    uint8_t           cont:1;      // 0 - Only packet, 1 continued packet following
    ///    uint8_t           id_width:1;  // 0 - 16bits, 1 - 32bits
    ///    uint8_t           reserved:1;  // Reserved
    ///    uint8_t           upper_len:2; // Upper 2 bits of length field (generally unused)
    ///    uint8_t           len;         // Lower 8 bits of length field
    ///    uint8_t           data[0];     // Start of data payload (may start with Id)
    /// };
    /// ```
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        // Check if buffer is ready to serialize
        if !self.done {
            return Err(ser::Error::custom("HIDIOPacketBuffer is not 'done'"));
        }

        // --- First Packet ---

        // Determine id_width
        let id_width: u8 = match (&self).id {
            0x00...0xFFFF =>         0, // 16 bit Id
            0x010000...0xFFFFFFFF => 1, // 32 bit Id
            _ => 0,
        };

        // Determine id_width_len
        let id_width_len: u8 = match id_width {
            0 => 2, // 2 bytes - 16 bit Id
            1 => 4, // 4 bytes - 32 bit Id
            _ => 0,
        };

        // Determine total header length, initial and continued packets (always 2 bytes)
        let hdr_len = 2 + id_width_len; // 1 byte for header, 1 byte for len, id_width_len for Id

        // Determine payload max length, initial and continued packets
        let payload_len = &self.max_len - hdr_len as u32;

        // Data length
        let data_len = (&self.data).len() as u32;

        // Determine if a continued packet construct
        let mut cont: bool = data_len > payload_len;

        // Determine packet len
        let packet_len: u16 = match cont {
            // Full payload length
            true  => payload_len as u16 + id_width_len as u16,
            // Calculate payload length with what's left
            false => data_len as u16 + id_width_len as u16,
        };

        // Determine upper_len and len fields
        let upper_len: u8 = (packet_len >> 8) as u8;
        let len: u8 = packet_len as u8;

        // Determine ptype
        let ptype: u8 = match (&self).ptype {
            HIDIOPacketType::Data      => 0,
            HIDIOPacketType::ACK       => 1,
            HIDIOPacketType::NAK       => 2,
            HIDIOPacketType::Sync      => 3,
            HIDIOPacketType::Continued => 4,
        };

        // Convert Id into bytes
        let mut id_vec: Vec<u8> = Vec::new();
        for idx in 0 .. id_width_len {
            let id = ((&self).id >> idx * 8) as u8;
            id_vec.push(id);
        }

        // Construct header byte
        let hdr_byte: u8 =
            // type - 3 bits
            (ptype << 5) |
            // cont - 1 bit
            (if cont { 1 } else { 0 } << 4) |
            // id_width - 1 bit
            (id_width << 3) |
            // reserved - 1 bit
            (0 << 2) |
            // upper_len - 2 bits
            (upper_len & 0x3);

        // Calculate total length of serialized output
        let serialized_len = ( data_len / payload_len ) * payload_len + data_len % payload_len + hdr_len as u32;

        // Serialize as a sequence
        let mut state = serializer.serialize_seq(Some(serialized_len as usize))?;

        // Serialize header
        state.serialize_element(&hdr_byte)?;

        // Serialize length
        state.serialize_element(&len)?;

        // If SYNC packet
        if self.ptype == HIDIOPacketType::Sync {
            return state.end();
        }

        // Serialize id
        for id_byte in &id_vec {
            state.serialize_element(id_byte)?;
        }

        // Serialize payload data
        // We can't just serialize directly (extra info is included), serialize each element of vector separately
        let slice = match cont {
            // Full payload length
            true =>  &self.data[0 .. payload_len as usize],
            // Payload that's available
            false => &self.data[0 .. data_len as usize],
        };
        for elem in slice {
            state.serialize_element(elem)?;
        }

        // Finish serialization if no more payload left
        if !cont {
            return state.end();
        }

        // Determine how much payload is left
        let mut payload_left = (&self.data).len() as u32 - payload_len;
        let mut last_slice_index = payload_len as usize;


        // --- Additional Packets ---

        while cont {
            // Determine if continued packet construct
            cont = payload_left > payload_len;

            // Continued Packet
            let ptype = 4; // HIDIOPacketType::Continued

            // Determine packet len
            let packet_len: u16 = match cont {
                // Full payload length
                true  => payload_len as u16 + id_width_len as u16,
                // Calculate payload length with what's left
                false => payload_left as u16 + id_width_len as u16,
            };

            // Determine upper_len and len fields
            let upper_len: u8 = (packet_len >> 8) as u8;
            let len: u8 = packet_len as u8;

            // Construct header byte
            let hdr_byte: u8 =
                // type - 3 bits
                (ptype << 5) |
                // cont - 1 bit
                (if cont { 1 } else { 0 } << 4) |
                // id_width - 1 bit
                (id_width << 3) |
                // reserved - 1 bit
                (0 << 2) |
                // upper_len - 2 bits
                (upper_len & 0x3);

            // Serialize header
            state.serialize_element(&hdr_byte)?;

            // Serialize length
            state.serialize_element(&len)?;

            // Serialize id
            for id_byte in &id_vec {
                state.serialize_element(id_byte)?;
            }

            // Serialize payload data
            // We can't just serialize directly (extra info is included), serialize each element of vector separately
            let slice_end = match cont {
                // Full payload length
                true =>  (last_slice_index + payload_len as usize),
                // Payload that's available
                false => data_len as usize,
            };
            let slice = match cont {
                // Full payload length
                true =>  &self.data[last_slice_index .. slice_end],
                // Payload that's available
                false => &self.data[last_slice_index .. slice_end],
            };
            for elem in slice {
                state.serialize_element(elem)?;
            }

            // Recalculate how much payload is left
            payload_left -= (slice_end - last_slice_index) as u32;
            last_slice_index += payload_len as usize;
        }


        // --- Finish serialization ---
        state.end()
    }
}

impl fmt::Display for HIDIOPacketType {
    /// Display formatter for HIDIOPacketType
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let ptype_name = match *self {
            HIDIOPacketType::Data      => "HIDIOPacketBuffer::Data",
            HIDIOPacketType::ACK       => "HIDIOPacketBuffer::ACK",
            HIDIOPacketType::NAK       => "HIDIOPacketBuffer::NAK",
            HIDIOPacketType::Sync      => "HIDIOPacketBuffer::Sync",
            HIDIOPacketType::Continued => "HIDIOPacketBuffer::Continued",
        };
        write!(f, "{}", ptype_name)
    }
}

impl fmt::Display for HIDIOPacketBuffer {
    /// Display formatter for HIDIOPacketBuffer
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\n{{\n    ptype: {}\n    id: {}\n    max_len: {}\n    done: {}\n    data: {:#?}\n}}",
            self.ptype,
            self.id,
            self.max_len,
            self.done,
            self.data,
        )
    }
}



// ----- Tests -----

#[cfg(test)]
mod test {
    use super::{HIDIOPacketBuffer, HIDIOPacketType};

    /// Loopback helper
    /// Serializes, deserializes, then checks if same as original
    fn loopback_serializer(mut buffer: HIDIOPacketBuffer) {
        // Serialize
        let serialized = match buffer.serialize_buffer() {
            Ok(result) => result,
            _ =>          Vec::new(),
        };

        // Validate serialization worked
        assert!(serialized.len() > 0, "Serialization bytes:{}", serialized.len());

        // Deserialize while there are bytes left
        let mut deserialized = HIDIOPacketBuffer::new();
        let mut bytes_used = 0;
        while bytes_used != serialized.len() {
            // Remove already processed bytes
            let slice = &serialized[bytes_used ..];
            match deserialized.decode_packet(&mut slice.to_vec()) {
                Ok(result) => {
                    bytes_used += result as usize;
                },
                _ => {
                    assert!(false, "Failured decoding packet");
                },
            };
        }

        // Set the max_len as decode_packet does not infer this (not enough information from datastream)
        deserialized.max_len = buffer.max_len;

        // Validate buffers are the same
        assert!(buffer == deserialized, "\nInput:{}\nSerialized:{:#?}\nOutput:{}", buffer, serialized, deserialized);

        // Validate all bytes used
        assert!(serialized.len() == bytes_used, "Serialized:{}, Deserialized Used:{}", serialized.len(), bytes_used);
    }


    /// Generates a single byte payload buffer
    /// Serializes, deserializes, then checks if same as original
    #[test]
    fn single_byte_payload_test() {
        // Create single byte payload buffer
        let buffer = HIDIOPacketBuffer {
            // Data packet
            ptype:   HIDIOPacketType::Data,
            // Test packet id
            id:      2,
            // Standard USB 2.0 FS packet length
            max_len: 64,
            // Single byte, 0xAC
            data:    vec![0xAC],
            // Ready to go
            done:    true,
        };

        // Run loopback serializer, handles all test validation
        loopback_serializer(buffer);
    }

    /// Generates a full packet payload buffer
    /// Serializes, deserializes, then checks if same as original
    #[test]
    fn full_packet_payload_test() {
        // Create single byte payload buffer
        let buffer = HIDIOPacketBuffer {
            // Data packet
            ptype:   HIDIOPacketType::Data,
            // Test packet id
            id:      2,
            // Standard USB 2.0 FS packet length
            max_len: 64,
            // 60 bytes, 0xAC; requires 2 byte header, and 2 bytes for id, which is 64 bytes
            data:    vec![0xAC; 60],
            // Ready to go
            done:    true,
        };

        // Run loopback serializer, handles all test validation
        loopback_serializer(buffer);
    }

    /// Generates a two packet payload buffer
    /// Serializes, deserializes, then checks if same as original
    #[test]
    fn two_packet_continued_payload_test() {
        // Create single byte payload buffer
        let buffer = HIDIOPacketBuffer {
            // Data packet
            ptype:   HIDIOPacketType::Data,
            // Test packet id
            id:      2,
            // Standard USB 2.0 FS packet length
            max_len: 64,
            // 110 bytes, 0xAC: 60 then 50 (62 then 52)
            data:    vec![0xAC; 110],
            // Ready to go
            done:    true,
        };

        // Run loopback serializer, handles all test validation
        loopback_serializer(buffer);
    }

    /// Generates a three packet payload buffer
    /// Serializes, deserializes, then checks if same as original
    #[test]
    fn three_packet_continued_payload_test() {
        // Create single byte payload buffer
        let buffer = HIDIOPacketBuffer {
            // Data packet
            ptype:   HIDIOPacketType::Data,
            // Test packet id
            id:      2,
            // Standard USB 2.0 FS packet length
            max_len: 64,
            // 170 bytes, 0xAC: 60, 60 then 50 (62, 62 then 52)
            data:    vec![0xAC; 170],
            // Ready to go
            done:    true,
        };

        // Run loopback serializer, handles all test validation
        loopback_serializer(buffer);
    }
}

