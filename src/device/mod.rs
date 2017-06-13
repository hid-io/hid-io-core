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

extern crate bincode;
extern crate serde;

use self::serde::ser::{Serialize, Serializer, SerializeStruct};
use self::bincode::{serialize, deserialize, Infinite};
use std::mem::transmute;

mod hidusb;

/// HID-IO Packet Types
#[derive(PartialEq)]
enum HIDIOPacketType {
    Data      = 0,
    ACK       = 1,
    NAK       = 2,
    Sync      = 3,
    Continued = 4,
}

/// HID-IO Packet Format Struct
/// TODO Bitfields
struct HIDIOPacket {
    ptype:   HIDIOPacketType, // Type of packet (Continued is automatically set if needed)
    max_len: u32,             // Maximum packet length, including header
    id:      u32,             // Packet Id
    data:    &'static [u8],   // Payload data, chunking is done automatically by serializer
}

/// Serializers
impl Serialize for HIDIOPacketType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        // TODO (HaaTa): Is there a better way to assign the values using the enum order?
        let val: u8 = match *self {
            HIDIOPacketType::Data      => 0,
            HIDIOPacketType::ACK       => 1,
            HIDIOPacketType::NAK       => 2,
            HIDIOPacketType::Sync      => 3,
            HIDIOPacketType::Continued => 4,
        };
        serializer.serialize_u8(val)
    }
}

impl Serialize for HIDIOPacket {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        // Determine cont, width, upper_len and len fields
        // This according to this C-Struct
        //
        // struct HIDIO_Packet {
        //    HIDIO_Packet_Type type:3;
        //    uint8_t           cont:1;      // 0 - Only packet, 1 continued packet following
        //    uint8_t           width:2;     // 0 - 8bits, 1 - 16bits, 2 - 24bits, 3 - 32bits
        //    uint8_t           upper_len:2; // Upper 2 bits of length field (generally unused)
        //    uint8_t           len;         // Lower 8 bits of length field
        //    uint8_t           id[0];       // Starting byte of id field (up to 4 bytes long, depending on width field)
        // };

        // --- First Packet ---

        // Determine width
        let width: u8 = match (&self).id {
            0x00...0xFF =>             0, // 8 bit Id
            0x0100...0xFFFF =>         1, // 16 bit Id
            0x010000...0xFFFFFF =>     2, // 24 bit Id
            0x01000000...0xFFFFFFFF => 3, // 32 bit Id
            _ => 0,
        };

        // Determine total header length, initial and continued packets (always 2 bytes)
        let hdr_len = 2 + width + 1; // 1 byte for header, 1 byte for len, width + 1 bytes for Id
        let hdr_len_con = 2;

        // Determine payload max length, initial and continued packets
        let mut payload_len = &self.max_len - hdr_len as u32;
        let payload_len_con = &self.max_len - hdr_len_con;

        // Determine if a continued packet construct
        let mut cont: bool = (&self.data).len() as u32 > payload_len;

        // Determine packet len
        let packet_len: u16 = match cont {
            true  => payload_len as u16,
            false => (&self).data.len() as u16 + width as u16 + 1,
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

        // Construct header byte
        let hdr_byte: u8 = (ptype << 5) | (if cont { 1 } else { 0 } << 4) | (width << 2) | upper_len;

        // Serialize first packet
        let mut state = serializer.serialize_struct("HIDIOPacket", 4)?;

        // XXX (HaaTa): Why do we need an extra 0x00 here?
        state.serialize_field("null", &(0 as u8));

        // Serialize header
        state.serialize_field("hdr", &hdr_byte);

        // If SYNC packet, only initial byte is needed
        if self.ptype == HIDIOPacketType::Sync {
            return state.end();
        }

        state.serialize_field("len", &len);

        // Stack Id
        for idx in 0 .. width + 1 {
            let id : u8 = (((&self).id & 0xFF) >> idx * 8) as u8;
            state.serialize_field("id", &id);
        }

        if cont {
            println!("Huzaaa");
            let slice = &self.data[0 .. payload_len as usize];
            // We can't just serialize directly (extra info is included), serialize each element of vector separately
            for elem in slice {
                state.serialize_field("data", elem);
            }
        } else {
            let slice = &self.data[0 .. (&self).data.len() as usize];
            // We can't just serialize directly (extra info is included), serialize each element of vector separately
            for elem in slice {
                state.serialize_field("data", elem);
            }
            payload_len = (&self.data).len() as u32;
        }

        // Determine how much payload is left
        let mut payload_left = (&self.data).len() as u32 - payload_len;
        let mut last_slice_index = payload_len;


        // --- Additional Packets ---

        while cont {
            // Determine if continued packet construct
            cont = payload_left > payload_len_con;

            // Continued Packet
            let ptype = 4; // HIDIOPacketType::Continued

            // Determine packet len
            let packet_len: u16 = match cont {
                true  => payload_len_con as u16,
                false => payload_left as u16,
            };

            // Determine upper_len and len fields
            let upper_len: u8 = (packet_len >> 8) as u8;
            let len: u8 = packet_len as u8;

            // Construct header byte
            let hdr_byte: u8 = (ptype << 5) | (if cont { 1 } else { 0 } << 4) | (width << 2) | upper_len;

            // Serialize next packet
            state.serialize_field("hdr", &hdr_byte);
            state.serialize_field("len", &len);
            if cont {
                let slice = &self.data[last_slice_index as usize .. (last_slice_index + payload_len_con) as usize];
                last_slice_index += payload_len_con;
                // We can't just serialize directly (extra info is included), serialize each element of vector separately
                for elem in slice {
                    state.serialize_field("data", elem);
                }
                payload_len = payload_len_con;
            } else {
                let slice = &self.data[last_slice_index as usize .. (&self.data).len()];
                // We can't just serialize directly (extra info is included), serialize each element of vector separately
                for elem in slice {
                    state.serialize_field("data", elem);
                }
                payload_len = (&self.data).len() as u32 - last_slice_index;
            }

            // Recalculate how much payload is left
            payload_left -= payload_len;
        }


        // --- Finish serialization ---
        state.end()
    }
}


/// Deserializers
// TODO


/// Create packet
pub fn packet_gen() -> Vec<u8> {
    let packet = HIDIOPacket {
        ptype:   HIDIOPacketType::Data,
        max_len: 64,
        id:      0x12,
        data:    b"yay! this is the way I can make a very long continued packet using rust!",
    };

    //let bytes = HIDIOPacket::serialize(&packet, Infinite).unwrap();
    let bytes: Vec<u8> = serialize(&packet, Infinite).unwrap();
    println!("{:?}", bytes);

    bytes
}


/// Module initialization
/// Sets up at least one thread per Device.
/// Each device is repsonsible for accepting and responding to packet requests.
/// It is also possible to send requests asynchronously back to any Modules.
/// Each device may have it's own RPC API.
pub fn initialize() {
    info!("Initializing devices...");

    // Initialize device watcher threads
    hidusb::initialize();
}

