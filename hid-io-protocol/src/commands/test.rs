/* Copyright (C) 2020-2021 by Jacob Alexander
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

// ----- Crates -----

use super::*;
use flexi_logger::Logger;
use heapless::consts::{U100, U150, U2, U3, U64, U8};

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

// ----- Structs -----

/// Test HID-IO Command Interface
struct CommandInterface<
    TX: ArrayLength<Vec<u8, N>>,
    RX: ArrayLength<Vec<u8, N>>,
    N: ArrayLength<u8>,
    H: ArrayLength<u8>,
    ID: ArrayLength<HidIoCommandID> + ArrayLength<u8> + Mul<U4> + Add<U5>,
> {
    ids: Vec<HidIoCommandID, ID>,
    rx_bytebuf: buffer::Buffer<RX, N>,
    rx_packetbuf: HidIoPacketBuffer<H>,
    tx_bytebuf: buffer::Buffer<TX, N>,
}

impl<
        TX: ArrayLength<Vec<u8, N>>,
        RX: ArrayLength<Vec<u8, N>>,
        N: ArrayLength<u8>,
        H: ArrayLength<u8>,
        ID: ArrayLength<HidIoCommandID> + ArrayLength<u8> + Mul<U4> + Add<U5>,
    > CommandInterface<TX, RX, N, H, ID>
{
    fn new(ids: &[HidIoCommandID]) -> Result<CommandInterface<TX, RX, N, H, ID>, CommandError> {
        // Make sure we have a large enough id vec
        let ids = match Vec::from_slice(ids) {
            Ok(ids) => ids,
            Err(_) => {
                return Err(CommandError::IdVecTooSmall);
            }
        };
        let tx_bytebuf = buffer::Buffer::new();
        let rx_bytebuf = buffer::Buffer::new();
        let rx_packetbuf = HidIoPacketBuffer::new();
        Ok(CommandInterface {
            ids,
            rx_bytebuf,
            rx_packetbuf,
            tx_bytebuf,
        })
    }

    /// Used to flush the tx_bytebuf into rx_bytebuf
    /// Effectively creates a loopback
    fn flush_tx2rx(&mut self) {
        while !self.tx_bytebuf.is_empty() {
            if let Some(data) = self.tx_bytebuf.dequeue() {
                self.rx_bytebuf.enqueue(data).unwrap();
            }
        }
    }
}

/// CommandInterface for Commands
/// NOTE: tx_bytebuf is a loopback buffer
///       rx_bytebuf just reads in tx_buf
impl<
        TX: ArrayLength<Vec<u8, N>>,
        RX: ArrayLength<Vec<u8, N>>,
        N: ArrayLength<u8>,
        H: ArrayLength<u8>,
        ID: ArrayLength<HidIoCommandID> + ArrayLength<u8> + Mul<U4> + Add<U5>,
    > Commands<TX, RX, N, H, ID> for CommandInterface<TX, RX, N, H, ID>
where
    <ID as Mul<U4>>::Output: Add<U5>,
    H: core::fmt::Debug,
{
    fn tx_bytebuffer(&mut self) -> &mut buffer::Buffer<TX, N> {
        &mut self.tx_bytebuf
    }
    fn rx_bytebuffer(&mut self) -> &mut buffer::Buffer<RX, N> {
        &mut self.rx_bytebuf
    }
    fn rx_packetbuffer(&self) -> &HidIoPacketBuffer<H> {
        &self.rx_packetbuf
    }
    fn rx_packetbuffer_mut(&mut self) -> &mut HidIoPacketBuffer<H> {
        &mut self.rx_packetbuf
    }
    fn rx_packetbuffer_set(&mut self, buf: HidIoPacketBuffer<H>) {
        self.rx_packetbuf = HidIoPacketBuffer {
            ptype: buf.ptype,
            id: buf.id,
            max_len: buf.max_len,
            data: buf.data,
            done: buf.done,
        };
    }
    fn rx_packetbuffer_clear(&mut self) {
        self.rx_packetbuf = HidIoPacketBuffer::new();
    }
    fn supported_id(&self, id: HidIoCommandID) -> bool {
        self.ids.iter().any(|&i| i == id)
    }

    fn h0000_supported_ids_cmd(&self, _data: h0000::Cmd) -> Result<h0000::Ack<ID>, h0000::Nak> {
        // Build id list to send back
        Ok(h0000::Ack::<ID> {
            ids: self.ids.clone(),
        })
    }
    fn h0000_supported_ids_ack(&self, data: h0000::Ack<ID>) -> Result<(), CommandError> {
        assert!(data.ids == self.ids);
        Ok(())
    }

    fn h0001_info_cmd(&self, data: h0001::Cmd) -> Result<h0001::Ack, h0001::Nak> {
        for entry in &H0001ENTRIES {
            if entry.property == data.property {
                return Ok(h0001::Ack {
                    property: data.property,
                    os: entry.os,
                    number: entry.number,
                    string: String::from(entry.string),
                });
            }
        }

        Err(h0001::Nak {
            property: data.property,
        })
    }
    fn h0001_info_ack(&self, data: h0001::Ack) -> Result<(), CommandError> {
        // Compare ack with entries
        for entry in &H0001ENTRIES {
            if entry.property == data.property
                && entry.os == data.os
                && entry.number == data.number
                && entry.string == data.string
            {
                return Ok(());
            }
        }

        Err(CommandError::InvalidProperty8(data.property as u8))
    }

    fn h0002_test_cmd(&self, data: h0002::Cmd<U256>) -> Result<h0002::Ack<U256>, h0002::Nak> {
        // Use first payload byte to lookup test entry
        // Then validate length
        let entry = &H0002ENTRIES[data.data[0] as usize];
        if entry.len == data.data.len() {
            Ok(h0002::Ack { data: data.data })
        } else {
            Err(h0002::Nak {})
        }
    }
    fn h0002_test_nacmd(&self, data: h0002::Cmd<U256>) -> Result<(), CommandError> {
        // Use first payload byte to lookup test entry
        // Then validate length
        let entry = &H0002ENTRIES[data.data[0] as usize];
        if entry.len == data.data.len() {
            Ok(())
        } else {
            Err(CommandError::TestFailure)
        }
    }
    fn h0002_test_ack(&self, data: h0002::Ack<U256>) -> Result<(), CommandError> {
        // Use first payload byte to lookup test entry
        // Then validate length
        let entry = &H0002ENTRIES[data.data[0] as usize];
        if entry.len == data.data.len() {
            Ok(())
        } else {
            Err(CommandError::TestFailure)
        }
    }
}

// VT TODO
// - Print buffer
//   * Size should be configurable at build time
// - Send message when buffer full, flush, or pattern found (\n)

// Event Buffers TODO
// - KLL Event buffer (maybe have some sort of generic buffer setup?)
// - Send buffer on each USB/Output processing cycle

#[test]
fn h0000_supported_ids_test() {
    setup_logging_lite().ok();

    // Build list of supported ids
    let ids = [
        HidIoCommandID::SupportedIDs,
        HidIoCommandID::GetInfo,
        HidIoCommandID::TestPacket,
    ];

    // Setup command interface
    let mut intf = CommandInterface::<U8, U8, U64, U100, U3>::new(&ids).unwrap();

    // Send command
    let send = intf.h0000_supported_ids(h0000::Cmd {});
    assert!(send.is_ok(), "h0000_supported_ids => {:?}", send);

    // Flush tx->rx
    intf.flush_tx2rx();

    // Process rx buffer
    let process = intf.process_rx(0);
    assert!(process.is_ok(), "process_rx1 => {:?}", process);

    // Flush tx->rx
    intf.flush_tx2rx();

    // Process rx buffer
    let process = intf.process_rx(0);
    assert!(process.is_ok(), "process_rx2 => {:?}", process);
}

// Build test entries
#[derive(Debug)]
struct H0001TestEntry<'a> {
    property: h0001::Property,
    os: h0001::OSType,
    number: u16,
    string: &'a str,
}
const H0001ENTRIES: [H0001TestEntry; 13] = [
    H0001TestEntry {
        property: h0001::Property::MajorVersion,
        os: h0001::OSType::Unknown,
        number: 12,
        string: "",
    },
    H0001TestEntry {
        property: h0001::Property::MinorVersion,
        os: h0001::OSType::Unknown,
        number: 34,
        string: "",
    },
    H0001TestEntry {
        property: h0001::Property::PatchVersion,
        os: h0001::OSType::Unknown,
        number: 79,
        string: "",
    },
    H0001TestEntry {
        property: h0001::Property::DeviceName,
        os: h0001::OSType::Unknown,
        number: 0,
        string: "My Device",
    },
    H0001TestEntry {
        property: h0001::Property::DeviceSerialNumber,
        os: h0001::OSType::Unknown,
        number: 0,
        string: "1234567890 - 0987654321",
    },
    H0001TestEntry {
        property: h0001::Property::DeviceVersion,
        os: h0001::OSType::Unknown,
        number: 0,
        string: "v9001",
    },
    H0001TestEntry {
        property: h0001::Property::DeviceMCU,
        os: h0001::OSType::Unknown,
        number: 0,
        string: "someMCUname",
    },
    H0001TestEntry {
        property: h0001::Property::FirmwareName,
        os: h0001::OSType::Unknown,
        number: 0,
        string: "SpecialDeviceFirmware",
    },
    H0001TestEntry {
        property: h0001::Property::FirmwareVersion,
        os: h0001::OSType::Unknown,
        number: 0,
        string: "v9999",
    },
    H0001TestEntry {
        property: h0001::Property::DeviceVendor,
        os: h0001::OSType::Unknown,
        number: 0,
        string: "HID-IO",
    },
    H0001TestEntry {
        property: h0001::Property::OsType,
        os: h0001::OSType::Linux,
        number: 0,
        string: "",
    },
    H0001TestEntry {
        property: h0001::Property::OsVersion,
        os: h0001::OSType::Unknown,
        number: 0,
        string: "Special Linux Version",
    },
    H0001TestEntry {
        property: h0001::Property::HostSoftwareName,
        os: h0001::OSType::Unknown,
        number: 0,
        string: "HID-IO Core Unit Test",
    },
];

#[test]
fn h0001_info_test() {
    setup_logging_lite().ok();

    // Build list of supported ids
    let ids = [HidIoCommandID::SupportedIDs, HidIoCommandID::GetInfo];

    // Setup command interface
    let mut intf = CommandInterface::<U8, U8, U64, U100, U2>::new(&ids).unwrap();

    // Process each of the test entries
    for entry in &H0001ENTRIES {
        // Send command
        let send = intf.h0001_info(h0001::Cmd {
            property: entry.property,
        });
        assert!(send.is_ok(), "h0001_info {:?} => {:?}", entry, send);

        // Flush tx->rx
        intf.flush_tx2rx();

        // Process rx buffer
        let process = intf.process_rx(0);
        assert!(process.is_ok(), "process_rx1 {:?} => {:?}", entry, process);

        // Flush tx->rx
        intf.flush_tx2rx();

        // Process rx buffer
        let process = intf.process_rx(0);
        assert!(process.is_ok(), "process_rx2 {:?} => {:?}", entry, process);
    }
}

// Build test entries
#[derive(Debug)]
struct H0002TestEntry {
    data: [u8; 128],
    len: usize,
}
const H0002ENTRIES: [H0002TestEntry; 4] = [
    // Small message
    H0002TestEntry {
        data: [0x00; 128],
        len: 1,
    },
    // Full message
    H0002TestEntry {
        data: [0x01; 128],
        len: 60,
    },
    // Multi-packet message
    H0002TestEntry {
        data: [0x02; 128],
        len: 61,
    },
    // Full multi-packet message
    H0002TestEntry {
        data: [0x03; 128],
        len: 120,
    },
];

#[test]
fn h0002_test_test() {
    setup_logging_lite().ok();

    // Build list of supported ids
    let ids = [
        HidIoCommandID::SupportedIDs,
        HidIoCommandID::GetInfo,
        HidIoCommandID::TestPacket,
    ];

    // Setup command interface
    let mut intf = CommandInterface::<U8, U8, U64, U150, U3>::new(&ids).unwrap();

    // Normal data packets
    for entry in &H0002ENTRIES {
        // Send command
        let mut cmd = h0002::Cmd { data: Vec::new() };
        for elem in 0..entry.len {
            cmd.data.push(entry.data[elem]).unwrap();
        }
        let send = intf.h0002_test(cmd, false);
        assert!(send.is_ok(), "h0002_test {:?} => {:?}", entry, send);

        // Flush tx->rx
        intf.flush_tx2rx();

        // Process rx buffer
        let process = intf.process_rx(0);
        assert!(process.is_ok(), "process_rx1 {:?} => {:?}", entry, process);

        // Flush tx->rx
        intf.flush_tx2rx();

        // Process rx buffer
        let process = intf.process_rx(0);
        assert!(process.is_ok(), "process_rx2 {:?} => {:?}", entry, process);
    }

    // NA (no-ack) data packets
    for entry in &H0002ENTRIES {
        // Send command
        let mut cmd = h0002::Cmd { data: Vec::new() };
        for elem in 0..entry.len {
            cmd.data.push(entry.data[elem]).unwrap();
        }
        let send = intf.h0002_test(cmd, true);
        assert!(send.is_ok(), "h0002_test(na) {:?} => {:?}", entry, send);

        // Flush tx->rx
        intf.flush_tx2rx();

        // Process rx buffer
        let process = intf.process_rx(0);
        assert!(process.is_ok(), "process_rx3 {:?} => {:?}", entry, process);
    }
}

#[test]
fn h0002_invalid_test() {
    setup_logging_lite().ok();

    // Build list of supported ids
    let ids = [HidIoCommandID::SupportedIDs, HidIoCommandID::GetInfo];

    // Setup command interface
    let mut intf = CommandInterface::<U8, U8, U64, U150, U2>::new(&ids).unwrap();

    // Send command
    let cmd = h0002::Cmd { data: Vec::new() };
    let send = intf.h0002_test(cmd, false);
    assert!(send.is_ok(), "h0002_invalid => {:?}", send);

    // Flush tx->rx
    intf.flush_tx2rx();

    // Process rx buffer (look for error)
    let process = intf.process_rx(0);
    assert!(process.is_err(), "process_rx1 => {:?}", process);

    // Send NA command
    let cmd = h0002::Cmd { data: Vec::new() };
    let send = intf.h0002_test(cmd, true);
    assert!(send.is_ok(), "h0002_invalid(na) => {:?}", send);

    // Flush tx->rx
    intf.flush_tx2rx();

    // Process rx buffer
    let process = intf.process_rx(0);
    assert!(process.is_err(), "process_rx2 => {:?}", process);
}

/*
#[test]
fn h0016_flashmode_test() {
    setup_logging_lite().ok();

    // TODO
    assert!(false, "BLA");
}

#[test]
fn h001A_sleepmode_test() {
    setup_logging_lite().ok();

    // TODO
    assert!(false, "BLA");
}

#[test]
fn h0031_terminalcmd_test() {
    setup_logging_lite().ok();

    // TODO
    assert!(false, "BLA");
}

#[test]
fn h0034_terminalout_test() {
    setup_logging_lite().ok();

    // TODO
    assert!(false, "BLA");
}
*/
