/* Copyright (C) 2017-2020 by Jacob Alexander
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in
 * all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
 * THE SOFTWARE.
 */

// ----- Crates -----

#![no_std]
#![feature(lang_items)]

// ----- Modules -----

use bincode_core::{serialize, BufferWriter};
use core::convert::TryFrom;
use core::fmt;
use heapless::consts::{U32, U4};
use heapless::ArrayLength;
use heapless::Vec;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use serde::ser::{self, Serialize, SerializeSeq, Serializer};

#[cfg(feature = "device")]
use core::panic::PanicInfo;

#[cfg(feature = "server")]
use log::{error, warn};

// ----- Macros -----

#[cfg(not(feature = "server"))]
macro_rules! warn {
    (target: $target:expr, $($arg:tt)+) => {};
    ($($arg:tt)+) => {};
}

#[cfg(not(feature = "server"))]
macro_rules! error {
    (target: $target:expr, $($arg:tt)+) => {};
    ($($arg:tt)+) => {};
}

// ----- Enumerations -----

/// HID-IO Packet Types
///
/// # Remarks
/// Must not be larger than 0x7, 7 is reserved.
#[derive(PartialEq, Clone, Copy, Debug)]
pub enum HidIoPacketType {
    /// Data packet
    Data = 0,
    /// ACK packet
    ACK = 1,
    /// NAK packet
    NAK = 2,
    /// Sync packet
    Sync = 3,
    /// Continued packet
    Continued = 4,
    /// No acknowledgement data packet
    NAData = 5,
    /// No acknowledgement continued packet
    NAContinued = 6,
}

#[repr(u32)]
#[derive(PartialEq, Clone, Copy, Debug, IntoPrimitive, TryFromPrimitive)]
/// Requests for to perform a specific action
pub enum HidIoCommandID {
    SupportedIDs = 0x00,
    GetInfo = 0x01,
    TestPacket = 0x02,
    ResetHidIo = 0x03,
    Reserved = 0x04, // ... 0x0F
    GetProperties = 0x10,
    KeyState = 0x11,
    KeyboardLayout = 0x12,
    KeyLayout = 0x13,
    KeyShapes = 0x14,
    LEDLayout = 0x15,
    FlashMode = 0x16,
    UnicodeText = 0x17,
    UnicodeKey = 0x18,
    HostMacro = 0x19,
    SleepMode = 0x1A,
    KLLState = 0x20,
    PixelSetting = 0x21,
    PixelSet1c8b = 0x22,
    PixelSet3c8b = 0x23,
    PixelSet1c16b = 0x24,
    PixelSet3c16b = 0x25,

    OpenURL = 0x30,
    Terminal = 0x31,
    GetInputLayout = 0x32,
    SetInputLayout = 0x33,

    HIDKeyboard = 0x40,
    HIDKeyboardLED = 0x41,
    HIDMouse = 0x42,
    HIDJoystick = 0x43,
    HIDSystemCtrl = 0x44,
    HIDConsumerCtrl = 0x45,

    ManufacturingTest = 0x50,

    Unused = 0xFFFF,
}

#[repr(u8)]
#[derive(PartialEq, Clone, Copy, Debug)]
/// Requests for a specific piece of info
pub enum HidIoPropertyID {
    HidIoMajor = 0x00,
    HidIoMinor = 0x01,
    HidIoPatch = 0x02,
    HostOS = 0x03,
    OSVersion = 0x04,
    HostName = 0x05,
}

#[repr(u8)]
#[derive(PartialEq, Clone, Copy, Debug)]
/// Platforms as a numeric id
pub enum HostOSID {
    Unknown = 0x00,
    Windows = 0x01,
    Linux = 0x02,
    Android = 0x03,
    Mac = 0x04,
    IOS = 0x05,
    ChromeOS = 0x06,
    FreeBSD = 0x07,
    OpenBSD = 0x08,
    NetBSD = 0x09,
}

// ----- Structs -----

/// HID-IO Packet Buffer Struct
///
/// # Remarks
/// Used to store HID-IO data chunks. Will be chunked into individual packets on transmission.
#[repr(C)]
#[derive(PartialEq, Clone, Debug)]
pub struct HidIoPacketBuffer<N: ArrayLength<u8>> {
    /// Type of packet (Continued is automatically set if needed)
    pub ptype: HidIoPacketType,
    /// Packet Id
    pub id: HidIoCommandID,
    /// Packet length for serialization (in bytes)
    pub max_len: u32,
    /// Payload data, chunking is done automatically by serializer
    pub data: Vec<u8, N>,
    /// Set False if buffer is not complete, True if it is
    pub done: bool,
}

/// HID-IO Parse Error
///
/// # Remarks
/// thrown when there's an issue processing byte stream.
#[derive(Debug)]
pub enum HidIoParseError {
    InvalidContinuedIdByte(u8),
    InvalidHidIoCommandID(u32),
    InvalidPacketIdWidth(u8),
    InvalidPacketType(u8),
    MissingContinuedIdByte,
    MissingPacketIdWidthByte,
    MissingPacketTypeByte,
    MissingPayloadLengthByte,
    NotEnoughActualBytesPacketId { len: usize, id_width: usize },
    NotEnoughPossibleBytesPacketId { len: u32, id_width: usize },
    PayloadAddFailed(usize),
    SerializationError,
    SerializationFailedResultTooSmall(usize),
    VecAddFailed,
    VecResizeFailed,
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
/// ```c
/// struct HidIo_Packet {
///    HidIo_Packet_Type type:3;
///    ...
/// };
/// ```
pub fn packet_type(packet_data: &[u8]) -> Result<HidIoPacketType, HidIoParseError> {
    let packet_data_len = packet_data.len();

    // Check if the byte stream is large enough
    if packet_data_len < 1 {
        return Err(HidIoParseError::MissingPacketTypeByte);
    }

    // Extract first 3 bits from first byte
    let ptype: u8 = (packet_data[0] & 0xE0) >> 5;

    // Convert to HidIoPacketType enum
    match ptype {
        0 => Ok(HidIoPacketType::Data),
        1 => Ok(HidIoPacketType::ACK),
        2 => Ok(HidIoPacketType::NAK),
        3 => Ok(HidIoPacketType::Sync),
        4 => Ok(HidIoPacketType::Continued),
        5 => Ok(HidIoPacketType::NAData),
        6 => Ok(HidIoPacketType::NAContinued),
        _ => Err(HidIoParseError::InvalidPacketType(ptype)),
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
/// ```c
/// struct HidIo_Packet {
///    ... (6 bits)
///    uint8_t           upper_len:2; // Upper 2 bits of length field (generally unused)
///    uint8_t           len;         // Lower 8 bits of length field
///    ...
/// };
pub fn payload_len(packet_data: &[u8]) -> Result<u32, HidIoParseError> {
    let packet_data_len = packet_data.len();

    // Check if the byte stream is large enough
    if packet_data_len < 2 {
        return Err(HidIoParseError::MissingPayloadLengthByte);
    }

    // Extract upper_len and len
    let upper_len = u32::from(packet_data[0] & 0x3);
    let len = u32::from(packet_data[1]);

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
/// ```c
/// struct HidIo_Packet {
///    ... (4 bits)
///    uint8_t           id_width:1;  // 0 - 16bits, 1 - 32bits
///    ...
/// };
pub fn packet_id_width(packet_data: &[u8]) -> Result<usize, HidIoParseError> {
    let packet_data_len = packet_data.len();

    // Check if the byte stream is large enough
    if packet_data_len < 2 {
        return Err(HidIoParseError::MissingPacketIdWidthByte);
    }

    // Extract id_width
    match packet_data[0] & 0x08 {
        0x00 => Ok(2), // 16 bit
        0x08 => Ok(4), // 32 bit
        _ => Err(HidIoParseError::InvalidPacketIdWidth(packet_data[0])),
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
/// ```c
/// struct HidIo_Packet {
///    ... (4 bits)
///    uint8_t           id_width:1;  // 0 - 16bits, 1 - 32bits
///    ... (11 bits)
///    uint16_t/uint32_t id;          // Id field (check id_width to see which struct to use)
///    ...
/// };
pub fn packet_id(packet_data: &[u8]) -> Result<u32, HidIoParseError> {
    let packet_data_len = packet_data.len();

    // Extract id_width
    let id_width = packet_id_width(packet_data)?;

    // Make sure there are enough possible bytes
    if payload_len(packet_data)? < id_width as u32 {
        return Err(HidIoParseError::NotEnoughPossibleBytesPacketId {
            len: payload_len(packet_data)?,
            id_width,
        });
    }

    // Make sure there enough actual bytes
    if packet_data_len < id_width + 2 {
        return Err(HidIoParseError::NotEnoughActualBytesPacketId {
            len: packet_data_len,
            id_width,
        });
    }

    // Iterate over bytes, constructing Id of either 16 or 32 bit width
    let mut id: u32 = 0;
    let offset = 2;
    for idx in 0..id_width as usize {
        id |= u32::from(packet_data[offset + idx]) << (idx * 8);
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
/// ```c
/// struct HidIo_Packet {
///    ... (3 bits)
///    uint8_t           cont:1;      // 0 - Only packet, 1 continued packet following
///    ...
/// };
pub fn continued_packet(packet_data: &[u8]) -> Result<bool, HidIoParseError> {
    let packet_data_len = packet_data.len() as u32;

    // Check if the byte stream is large enough
    if packet_data_len < 1 {
        return Err(HidIoParseError::MissingContinuedIdByte);
    }

    // Extract cont field
    // Determine value
    match packet_data[0] & 0x10 {
        0x10 => Ok(true),
        0x00 => Ok(false),
        _ => Err(HidIoParseError::InvalidContinuedIdByte(packet_data[0])),
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
pub fn payload_start(packet_data: &[u8]) -> Result<usize, HidIoParseError> {
    // Retrieve id_width
    let id_width = packet_id_width(packet_data)?;

    // Retrieve payload_len, if 0, then return 2 (minimum packet size)
    if payload_len(packet_data)? == 0 {
        return Ok(2);
    }

    // Determine starting position
    Ok(2 + id_width as usize)
}

// ----- Command Utility Functions -----

/// Converts a HID bitmask into an array of byte codes
///
/// # Arguments
/// * `bitmask` - Vector of bytes (each byte is an 8 bit bitmask)
///
/// # Remarks
/// The very first byte in the bitmask represents 0->7 and the final byte ends at 255
/// Opposite of keyboard_vec2bitmask.
/// NOTE: The vector is currently restricted to 32 byte codes
///       technically this could be a maximum of 256, but that
///       is both impractical and unlikely. i.e. we only have 10
///       fingers.
pub fn hid_bitmask2vec(bitmask: &[u8]) -> Result<Vec<u8, U32>, HidIoParseError> {
    let mut data: Vec<u8, U32> = Vec::new();

    // Iterate over each byte of the bitmask adding a code for each found bit
    for (byte_pos, byte) in bitmask.iter().enumerate() {
        // Iterate over each of the bits
        for b in 0..=7 {
            // Check if bit is active, if so use the b position, then add byte_pos
            let active = ((byte >> b) & 0x01) == 0x01;
            if active {
                let code = b + byte_pos * 8;
                if data.push(code as u8).is_err() {
                    return Err(HidIoParseError::VecAddFailed);
                }
            }
        }
    }
    Ok(data)
}

/// Converts a HID byte code array into a bitmask
///
/// # Arguments
/// * `codes` - Vector of bytes (e.g. each byte is a HID keyboard code)
///
/// # Remarks
/// Opposite of keyboard_bitmask2vec.
pub fn hid_vec2bitmask(codes: &[u8]) -> Result<Vec<u8, U32>, HidIoParseError> {
    let mut data: Vec<u8, U32> = Vec::new(); // Maximum of 32 bytes when dealing with 8 bit codes
    if data.resize_default(32).is_err() {
        return Err(HidIoParseError::VecResizeFailed);
    }

    // Iterate through codes and set each bit accordingly
    for code in codes {
        let byte_pos = code / 8; // Determine which byte
        let bit_mask = 1 << (code - 8 * byte_pos); // Determine which bit
        data[byte_pos as usize] |= bit_mask;
    }
    Ok(data)
}

// ----- Implementations -----

impl<N> Default for HidIoPacketBuffer<N>
where
    N: ArrayLength<u8>,
{
    fn default() -> Self {
        HidIoPacketBuffer {
            ptype: HidIoPacketType::Data,
            id: HidIoCommandID::try_from(0).unwrap(),
            max_len: 0,
            data: Vec::new(),
            done: false,
        }
    }
}

impl<N: ArrayLength<u8>> HidIoPacketBuffer<N> {
    /// Constructor for HidIoPacketBuffer
    ///
    /// # Remarks
    /// Initialize as blank
    pub fn new() -> HidIoPacketBuffer<N> {
        HidIoPacketBuffer {
            ..Default::default()
        }
    }

    /// Append payload data
    ///
    /// # Arguments
    /// * `new_data` - Vector of bytes
    ///
    /// # Remarks
    /// Appends payload to HidIoPacketBuffer.
    pub fn append_payload(&mut self, new_data: &[u8]) -> bool {
        // Check if buffer was already finished
        if !self.done {
            warn!("HidIoPacketBuffer is already 'done'");
            return false;
        }

        self.data.extend_from_slice(new_data).is_ok()
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
    pub fn decode_packet(&mut self, packet_data: &[u8]) -> Result<u32, HidIoParseError> {
        // Check if buffer was already finished
        if self.done {
            warn!("HidIoPacketBuffer is already 'done'");
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
            warn!(
                "Dropping. Not enough bytes available in packet stream. got:{}, expected:{}",
                packet_data_len - 2,
                payload_len
            );
            return Ok(packet_data_len);
        }

        // Get packet Id
        let id_num = packet_id(packet_data)?;
        let id = match HidIoCommandID::try_from(id_num) {
            Ok(id) => id,
            Err(e) => {
                error!("Failed to convert {} to HidIoCommandID: {}", id_num, e);
                return Err(HidIoParseError::InvalidHidIoCommandID(id_num));
            }
        };

        // Is this a new packet?
        // More information to set, if initializing buffer
        if self.data.is_empty()
            && (ptype != HidIoPacketType::Continued && ptype != HidIoPacketType::NAContinued)
        {
            // Set packet type
            self.ptype = ptype;

            // Set packet id
            self.id = id;

        // Make sure the current buffer matches what we're expecting
        } else {
            // Check for invalid packet type
            if self.data.is_empty() && ptype == HidIoPacketType::Continued {
                warn!("Dropping. Invalid packet type when initializing buffer, HidIoPacketType::Continued");
                return Ok(packet_len);
            }
            if self.data.is_empty() && ptype == HidIoPacketType::NAContinued {
                warn!("Dropping. Invalid packet type when initializing buffer, HidIoPacketType::NAContinued");
                return Ok(packet_len);
            }

            // Check if not a continued packet, and we have a payload
            if !(self.data.is_empty()
                || ptype == HidIoPacketType::Continued
                || ptype == HidIoPacketType::NAContinued)
            {
                warn!("Dropping. Invalid packet type (non-HidIoPacketType::Continued) on a already initialized buffer");
                return Ok(packet_len);
            }

            // Validate that we're looking at the same Id
            if self.id != id {
                warn!(
                    "Dropping. Invalid incoming id:{:?}, expected:{:?}",
                    id, self.id
                );
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
        let slice =
            &packet_data[payload_start..payload_start + payload_len as usize - id_width_len];
        match self.data.extend_from_slice(slice) {
            Ok(_) => {}
            Err(_) => {
                return Err(HidIoParseError::PayloadAddFailed(slice.len()));
            }
        }

        // Finished
        Ok(packet_len)
    }

    /// Serialize HidIoPacketBuffer
    ///
    /// # Remarks
    /// Provides a raw data vector to the serialized data.
    /// Removes some of the header that Serialize from serde prepends.
    pub fn serialize_buffer<'a>(
        &mut self,
        data: &'a mut [u8],
    ) -> Result<&'a [u8], HidIoParseError> {
        let options = bincode_core::config::DefaultOptions::new();
        let mut writer = BufferWriter::new(data);
        let len;

        // Serialize
        match serialize(&self, &mut writer, options) {
            Ok(_) => {}
            Err(e) => {
                error!("Parse error: {:?}", e);
                return Err(HidIoParseError::SerializationError);
            }
        };

        // Make sure serialization worked
        len = writer.written_len();
        if len < 5 {
            error!(
                "Serialization too small: {} -> {:02X?}",
                len,
                writer.written_buffer()
            );
            return Err(HidIoParseError::SerializationFailedResultTooSmall(len));
        }

        // Slice off the first byte (length) header bytes from serde
        let slice = &data[1..len as usize];
        Ok(slice)
    }
}

impl<N> Serialize for HidIoPacketBuffer<N>
where
    N: ArrayLength<u8>,
{
    /// Serializer for HidIoPacketBuffer
    ///
    /// # Remarks
    /// Determine cont, width, upper_len and len fields
    /// According to this C-Struct:
    ///
    /// ```c
    /// struct HidIo_Packet {
    ///    HidIo_Packet_Type type:3;
    ///    uint8_t           cont:1;      // 0 - Only packet, 1 continued packet following
    ///    uint8_t           id_width:1;  // 0 - 16bits, 1 - 32bits
    ///    uint8_t           reserved:1;  // Reserved
    ///    uint8_t           upper_len:2; // Upper 2 bits of length field (generally unused)
    ///    uint8_t           len;         // Lower 8 bits of length field
    ///    uint8_t           data[0];     // Start of data payload (may start with Id)
    /// };
    /// ```
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Check if buffer is ready to serialize
        if !self.done {
            return Err(ser::Error::custom("HidIoPacketBuffer is not 'done'"));
        }

        // --- First Packet ---

        // Determine id_width
        let id_width: u8 = match self.id as u32 {
            0x00..=0xFFFF => 0,           // 16 bit Id
            0x01_0000..=0xFFFF_FFFF => 1, // 32 bit Id
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
        let payload_len = self.max_len - u32::from(hdr_len);

        // Data length
        let data_len = (&self.data).len() as u32;

        // Determine if a continued packet construct
        let mut cont: bool = data_len > payload_len;

        // Determine packet len
        let packet_len: u16 = if cont {
            // Full payload length
            payload_len as u16 + u16::from(id_width_len)
        } else {
            // Calculate payload length with what's left
            data_len as u16 + u16::from(id_width_len)
        };

        // Determine upper_len and len fields
        let upper_len: u8 = (packet_len >> 8) as u8;
        let len: u8 = packet_len as u8;

        // Determine ptype
        let ptype: u8 = match self.ptype {
            HidIoPacketType::Data => 0,
            HidIoPacketType::ACK => 1,
            HidIoPacketType::NAK => 2,
            HidIoPacketType::Sync => 3,
            HidIoPacketType::Continued => 4,
            HidIoPacketType::NAData => 5,
            HidIoPacketType::NAContinued => 6,
        };

        // Convert Id into bytes
        let mut id_vec: Vec<u8, U4> = Vec::new();
        for idx in 0..id_width_len {
            let id = (self.id as u32 >> (idx * 8)) as u8;
            if id_vec.push(id).is_err() {
                return Err(ser::Error::custom(
                    "HidIoPacketBuffer failed to convert Id into bytes, vec add failed.",
                ));
            }
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
            // (0 << 2) |
            // upper_len - 2 bits
            (upper_len & 0x3);

        // Calculate total length of serialized output
        let serialized_len =
            (data_len / payload_len) * payload_len + data_len % payload_len + u32::from(hdr_len);

        // Serialize as a sequence
        let mut state = serializer.serialize_seq(Some(serialized_len as usize))?;

        // Serialize header
        state.serialize_element(&hdr_byte)?;

        // Serialize length
        state.serialize_element(&len)?;

        // If SYNC packet
        if self.ptype == HidIoPacketType::Sync {
            return state.end();
        }

        // Serialize id
        for id_byte in &id_vec {
            state.serialize_element(id_byte)?;
        }

        // Serialize payload data
        // We can't just serialize directly (extra info is included), serialize each element of vector separately
        let slice = if cont {
            // Full payload length
            &self.data[0..payload_len as usize]
        } else {
            // Payload that's available
            &self.data[0..data_len as usize]
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
            let ptype = 4; // HidIoPacketType::Continued

            // Determine packet len
            let packet_len: u16 = if cont {
                // Full payload length
                payload_len as u16 + u16::from(id_width_len)
            } else {
                // Calculate payload length with what's left
                payload_left as u16 + u16::from(id_width_len)
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
                // (0 << 2) |
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
            let slice_end = if cont {
                // Full payload length
                last_slice_index + payload_len as usize
            } else {
                // Payload that's available
                data_len as usize
            };
            let slice = &self.data[last_slice_index..slice_end];
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

impl fmt::Display for HidIoPacketType {
    /// Display formatter for HidIoPacketType
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ptype_name = match *self {
            HidIoPacketType::Data => "HidIoPacketBuffer::Data",
            HidIoPacketType::ACK => "HidIoPacketBuffer::ACK",
            HidIoPacketType::NAK => "HidIoPacketBuffer::NAK",
            HidIoPacketType::Sync => "HidIoPacketBuffer::Sync",
            HidIoPacketType::Continued => "HidIoPacketBuffer::Continued",
            HidIoPacketType::NAData => "HidIoPacketBuffer::NAData",
            HidIoPacketType::NAContinued => "HidIoPacketBuffer::NAContinued",
        };
        write!(f, "{}", ptype_name)
    }
}

impl<N: ArrayLength<u8>> fmt::Display for HidIoPacketBuffer<N> {
    /// Display formatter for HidIoPacketBuffer
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "\n{{\n    ptype: {}\n    id: {:?}\n    max_len: {}\n    done: {}\n    data: {:#?}\n}}",
            self.ptype, self.id, self.max_len, self.done, self.data,
        )
    }
}

//#[cfg(all(not(test), target_feature = "thumb-mode"))]
#[cfg(all(not(test), feature = "device"))]
#[lang = "eh_personality"]
fn eh_personality() {}

//#[cfg(all(not(test), target_feature = "thumb-mode"))]
#[cfg(all(not(test), feature = "device"))]
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

// ----- Tests -----

#[cfg(test)]
mod test {
    use super::*;
    use flexi_logger::Logger;
    use heapless::consts::{U1, U110, U170, U60, U7};

    enum LogError {
        CouldNotStartLogger,
    }

    /// Lite logging setup
    fn setup_logging_lite() -> Result<(), LogError> {
        match Logger::with_env_or_str("")
            .format(flexi_logger::colored_default_format)
            .format_for_files(flexi_logger::colored_detailed_format)
            .duplicate_to_stderr(flexi_logger::Duplicate::All)
            .start()
        {
            Err(_) => Err(LogError::CouldNotStartLogger),
            Ok(_) => Ok(()),
        }
    }

    /// Loopback helper
    /// Serializes, deserializes, then checks if same as original
    fn loopback_serializer<N: ArrayLength<u8> + core::cmp::PartialEq>(
        mut buffer: HidIoPacketBuffer<N>,
        data: &mut [u8],
    ) {
        // Serialize
        let data = match buffer.serialize_buffer(data) {
            Ok(data) => data,
            Err(err) => {
                assert!(false, "Serialized Buffer failed: {:?}", err);
                &[0u8; 0]
            }
        };

        // Validate serialization worked
        assert!(data.len() > 0, "Serialization bytes:{}", data.len());

        // Deserialize while there are bytes left
        let mut deserialized = HidIoPacketBuffer::new();
        let mut bytes_used = 0;
        while bytes_used != data.len() {
            // Remove already processed bytes
            let slice = &data[bytes_used..];
            match deserialized.decode_packet(&mut slice.to_vec()) {
                Ok(result) => {
                    bytes_used += result as usize;
                }
                _ => {
                    panic!("Failured decoding packet");
                }
            };
        }

        // Set the max_len as decode_packet does not infer this (not enough information from datastream)
        deserialized.max_len = buffer.max_len;

        // Validate buffers are the same
        assert!(
            buffer == deserialized,
            "\nInput:{}\nSerialized:{:#?}\nOutput:{}",
            buffer,
            data.len(),
            deserialized
        );

        // Validate all bytes used
        assert!(
            data.len() == bytes_used,
            "Serialized:{}, Deserialized Used:{}",
            data.len(),
            bytes_used
        );
    }

    /// Generates a single byte payload buffer
    /// Serializes, deserializes, then checks if same as original
    #[test]
    fn single_byte_payload_test() {
        setup_logging_lite().ok();

        // Create single byte payload buffer
        let buffer = HidIoPacketBuffer::<U1> {
            // Data packet
            ptype: HidIoPacketType::Data,
            // Test packet id
            id: HidIoCommandID::TestPacket,
            // Standard USB 2.0 FS packet length
            max_len: 64,
            // Single byte, 0xAC
            data: Vec::from_slice(&[0xAC]).unwrap(),
            // Ready to go
            done: true,
        };

        // Run loopback serializer, handles all test validation
        let mut data = [0u8; 64];
        loopback_serializer(buffer, &mut data);
    }

    /// Generates a full packet payload buffer
    /// Serializes, deserializes, then checks if same as original
    #[test]
    fn full_packet_payload_test() {
        setup_logging_lite().ok();

        // Create single byte payload buffer
        let buffer = HidIoPacketBuffer::<U60> {
            // Data packet
            ptype: HidIoPacketType::Data,
            // Test packet id
            id: HidIoCommandID::TestPacket,
            // Standard USB 2.0 FS packet length
            max_len: 64,
            // 60 bytes, 0xAC; requires 2 byte header, and 2 bytes for id, which is 64 bytes
            data: Vec::from_slice(&[0xAC; 60]).unwrap(),
            // Ready to go
            done: true,
        };

        // Run loopback serializer, handles all test validation
        let mut data = [0u8; 65];
        loopback_serializer(buffer, &mut data);
    }

    /// Generates a two packet payload buffer
    /// Serializes, deserializes, then checks if same as original
    #[test]
    fn two_packet_continued_payload_test() {
        setup_logging_lite().ok();

        // Create single byte payload buffer
        let buffer = HidIoPacketBuffer::<U110> {
            // Data packet
            ptype: HidIoPacketType::Data,
            // Test packet id
            id: HidIoCommandID::TestPacket,
            // Standard USB 2.0 FS packet length
            max_len: 64,
            // 110 bytes, 0xAC: 60 then 50 (62 then 52)
            data: Vec::from_slice(&[0xAC; 110]).unwrap(),
            // Ready to go
            done: true,
        };

        // Run loopback serializer, handles all test validation
        let mut data = [0u8; 128];
        loopback_serializer(buffer, &mut data);
    }

    /// Generates a three packet payload buffer
    /// Serializes, deserializes, then checks if same as original
    #[test]
    fn three_packet_continued_payload_test() {
        setup_logging_lite().ok();

        // Create single byte payload buffer
        let buffer = HidIoPacketBuffer::<U170> {
            // Data packet
            ptype: HidIoPacketType::Data,
            // Test packet id
            id: HidIoCommandID::TestPacket,
            // Standard USB 2.0 FS packet length
            max_len: 64,
            // 170 bytes, 0xAC: 60, 60 then 50 (62, 62 then 52)
            data: Vec::from_slice(&[0xAC; 170]).unwrap(),
            // Ready to go
            done: true,
        };

        // Run loopback serializer, handles all test validation
        let mut data = [0u8; 200];
        loopback_serializer(buffer, &mut data);
    }

    /// Tests hid_bitmask2vec and hid_vec2bitmask
    #[test]
    fn hid_vec2bitmask2vec_test() {
        setup_logging_lite().ok();

        let inputvec: Vec<u8, U7> = Vec::from_slice(&[1, 2, 3, 4, 5, 100, 255]).unwrap();

        // Convert, then convert back
        let bitmask = match hid_vec2bitmask(&inputvec) {
            Ok(bitmask) => bitmask,
            Err(e) => {
                assert!(false, "Failed to run hid_vec2bitmask: {:?}", e);
                Vec::new()
            }
        };
        let new_vec = match hid_bitmask2vec(&bitmask) {
            Ok(new_vec) => new_vec,
            Err(e) => {
                assert!(false, "Failed to run hid_bitmask2vec: {:?}", e);
                Vec::new()
            }
        };

        // Compare with original
        assert_eq!(
            inputvec, new_vec,
            "Bitmask test failed! Input: {:?}\nOutput: {:?}",
            inputvec, new_vec,
        );
    }
}
