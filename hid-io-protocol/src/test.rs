/* Copyright (C) 2017-2021 by Jacob Alexander
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

#![cfg(test)]

// ----- Modules -----

use super::*;
use flexi_logger::Logger;

// ----- Enumerations -----

enum LogError {
    CouldNotStartLogger,
}

// ----- Functions -----

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
fn loopback_serializer<const H: usize>(mut buffer: HidIoPacketBuffer<H>, data: &mut [u8]) {
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

// ----- Tests -----

/// Generates a sync payload and attempts to serialize
/// This is the simplest hid-io packet
/// Serializes, deserializes, then checks if same as original
#[test]
fn sync_payload_test() {
    setup_logging_lite().ok();

    // Create single byte payload buffer
    let buffer = HidIoPacketBuffer::<1> {
        // Data packet
        ptype: HidIoPacketType::Sync,
        // Ready to go
        done: true,
        // Use defaults for other fields (unused)
        ..Default::default()
    };

    // Run loopback serializer, handles all test validation
    let mut data = [0u8; 2];
    loopback_serializer(buffer, &mut data);
}

/// Zero byte data payload
/// This is the simplest data packet
/// Serializes, deserializes, then checks if same as original
#[test]
fn no_payload_test() {
    setup_logging_lite().ok();

    // Create single byte payload buffer
    let buffer = HidIoPacketBuffer::<0> {
        // Data packet
        ptype: HidIoPacketType::Data,
        // Test packet id
        id: HidIoCommandId::TestPacket,
        // Standard USB 2.0 FS packet length
        max_len: 64,
        // No payload
        data: Vec::new(),
        // Ready to go
        done: true,
    };

    // Run loopback serializer, handles all test validation
    let mut data = [0u8; 5];
    loopback_serializer(buffer, &mut data);
}

/// Generates a single byte payload buffer
/// Serializes, deserializes, then checks if same as original
#[test]
fn single_byte_payload_test() {
    setup_logging_lite().ok();

    // Create single byte payload buffer
    let buffer = HidIoPacketBuffer::<1> {
        // Data packet
        ptype: HidIoPacketType::Data,
        // Test packet id
        id: HidIoCommandId::TestPacket,
        // Standard USB 2.0 FS packet length
        max_len: 64,
        // Single byte, 0xAC
        data: Vec::from_slice(&[0xAC]).unwrap(),
        // Ready to go
        done: true,
    };

    // Run loopback serializer, handles all test validation
    let mut data = [0u8; 6];
    loopback_serializer(buffer, &mut data);
}

/// Generates a full packet payload buffer
/// Serializes, deserializes, then checks if same as original
#[test]
fn full_packet_payload_test() {
    setup_logging_lite().ok();

    // Create single byte payload buffer
    let buffer = HidIoPacketBuffer::<60> {
        // Data packet
        ptype: HidIoPacketType::Data,
        // Test packet id
        id: HidIoCommandId::TestPacket,
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
    let buffer = HidIoPacketBuffer::<110> {
        // Data packet
        ptype: HidIoPacketType::Data,
        // Test packet id
        id: HidIoCommandId::TestPacket,
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
    let buffer = HidIoPacketBuffer::<170> {
        // Data packet
        ptype: HidIoPacketType::Data,
        // Test packet id
        id: HidIoCommandId::TestPacket,
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

/// Generates a serialized length greater than 1 byte (255)
#[test]
fn four_packet_continued_payload_test() {
    setup_logging_lite().ok();

    // Create single byte payload buffer
    let buffer = HidIoPacketBuffer::<240> {
        // Data packet
        ptype: HidIoPacketType::Data,
        // Test packet id
        id: HidIoCommandId::TestPacket,
        // Standard USB 2.0 FS packet length
        max_len: 64,
        // 240 bytes, 0xAC: 60, 60, 60 then 60 (64, 64, 64, 64)
        data: Vec::from_slice(&[0xAC; 240]).unwrap(),
        // Ready to go
        done: true,
    };

    // Run loopback serializer, handles all test validation
    let mut data = [0u8; 257];
    loopback_serializer(buffer, &mut data);
}

/// Tests hid_bitmask2vec and hid_vec2bitmask
#[test]
fn hid_vec2bitmask2vec_test() {
    setup_logging_lite().ok();

    let inputvec: Vec<u8, 7> = Vec::from_slice(&[1, 2, 3, 4, 5, 100, 255]).unwrap();

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
