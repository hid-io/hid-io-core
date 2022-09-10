/* Copyright (C) 2020-2022 by Jacob Alexander
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

#[cfg(feature = "server")]
use log::debug;

#[cfg(feature = "device")]
use defmt::{debug, error};

// ----- Enumerations -----

enum LogError {
    CouldNotStartLogger,
}

// ----- Functions -----

/// Lite logging setup
fn setup_logging_lite() -> Result<(), LogError> {
    match Logger::try_with_env_or_str("")
        .unwrap()
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
    const TX: usize,
    const RX: usize,
    const N: usize,
    const H: usize,
    const HSUB1: usize,
    const HSUB2: usize,
    const HSUB4: usize,
    const S: usize,
    const ID: usize,
> {
    ids: Vec<HidIoCommandId, ID>,
    rx_bytebuf: buffer::Buffer<RX, N>,
    rx_packetbuf: HidIoPacketBuffer<H>,
    tx_bytebuf: buffer::Buffer<TX, N>,
    serial_buf: Vec<u8, S>,
}

impl<
        const TX: usize,
        const RX: usize,
        const N: usize,
        const H: usize,
        const HSUB1: usize,
        const HSUB2: usize,
        const HSUB4: usize,
        const S: usize,
        const ID: usize,
    > CommandInterface<TX, RX, N, H, HSUB1, HSUB2, HSUB4, S, ID>
{
    fn new(
        ids: &[HidIoCommandId],
    ) -> Result<CommandInterface<TX, RX, N, H, HSUB1, HSUB2, HSUB4, S, ID>, CommandError> {
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
        let serial_buf = Vec::new();
        Ok(CommandInterface {
            ids,
            rx_bytebuf,
            rx_packetbuf,
            tx_bytebuf,
            serial_buf,
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

    /// Decode rx_bytebuf into a HidIoPacketBuffer
    /// Returns true if buffer ready, false if not
    fn rx_packetbuffer_decode(&mut self) -> Result<bool, CommandError> {
        loop {
            // Retrieve vec chunk
            if let Some(buf) = self.rx_bytebuf.dequeue() {
                // Decode chunk
                match self.rx_packetbuf.decode_packet(&buf) {
                    Ok(_recv) => {
                        // Only handle buffer if ready
                        if self.rx_packetbuf.done {
                            // Handle sync packet type
                            match self.rx_packetbuf.ptype {
                                HidIoPacketType::Sync => {
                                    debug!("Sync. Resetting buffer");
                                    self.rx_packetbuf.clear();
                                }
                                _ => {
                                    return Ok(true);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("Decode error: {:?} {:?}", e, buf);
                        return Err(CommandError::PacketDecodeError(e));
                    }
                }
            } else {
                return Ok(false);
            }
        }
    }

    /// Process rx buffer until empty
    /// Handles flushing tx->rx, decoding, then processing buffers
    fn process_rx(&mut self) -> Result<(), CommandError> {
        // Flush tx->rx
        self.flush_tx2rx();

        // Decode bytes into buffer
        while self.rx_packetbuffer_decode()? {
            // Process rx buffer
            self.rx_message_handling(self.rx_packetbuf.clone())?;

            // Clear buffer
            self.rx_packetbuf.clear();
        }

        Ok(())
    }
}

/// CommandInterface for Commands
/// NOTE: tx_bytebuf is a loopback buffer
///       rx_bytebuf just reads in tx_buf
/// TX - tx byte buffer size (in multiples of N)
/// RX - tx byte buffer size (in multiples of N)
/// N - Max payload length (HidIoPacketBuffer), used for default values
/// H - Max data payload length (HidIoPacketBuffer)
/// HSUB1, HSUB4 - Due to current limitations of const generics
/// S - Serialization buffer size
/// ID - Max number of HidIoCommandIds
impl<
        const TX: usize,
        const RX: usize,
        const N: usize,
        const H: usize,
        const HSUB1: usize,
        const HSUB2: usize,
        const HSUB4: usize,
        const S: usize,
        const ID: usize,
    > Commands<H, HSUB1, HSUB2, HSUB4, ID>
    for CommandInterface<TX, RX, N, H, HSUB1, HSUB2, HSUB4, S, ID>
{
    fn default_packet_chunk(&self) -> u32 {
        N as u32
    }

    fn tx_packetbuffer_send(&mut self, buf: &mut HidIoPacketBuffer<H>) -> Result<(), CommandError> {
        let size = buf.serialized_len() as usize;
        if self.serial_buf.resize_default(size).is_err() {
            return Err(CommandError::SerializationVecTooSmall);
        }
        let data = match buf.serialize_buffer(&mut self.serial_buf) {
            Ok(data) => data,
            Err(err) => {
                return Err(CommandError::SerializationFailed(err));
            }
        };

        // Add serialized data to buffer
        // May need to enqueue multiple packets depending how much
        // was serialized
        for pos in (0..data.len()).step_by(N) {
            let len = core::cmp::min(N, data.len() - pos);
            match self
                .tx_bytebuf
                .enqueue(match Vec::from_slice(&data[pos..len + pos]) {
                    Ok(vec) => vec,
                    Err(_) => {
                        return Err(CommandError::TxBufferVecTooSmall);
                    }
                }) {
                Ok(_) => {}
                Err(_) => {
                    return Err(CommandError::TxBufferSendFailed);
                }
            }
        }
        Ok(())
    }
    fn supported_id(&self, id: HidIoCommandId) -> bool {
        self.ids.iter().any(|&i| i == id)
    }

    fn h0000_supported_ids_cmd(&mut self, _data: h0000::Cmd) -> Result<h0000::Ack<ID>, h0000::Nak> {
        // Build id list to send back
        Ok(h0000::Ack::<ID> {
            ids: self.ids.clone(),
        })
    }
    fn h0000_supported_ids_ack(&mut self, data: h0000::Ack<ID>) -> Result<(), CommandError> {
        assert!(data.ids == self.ids);
        Ok(())
    }

    fn h0001_info_cmd(&mut self, data: h0001::Cmd) -> Result<h0001::Ack<HSUB1>, h0001::Nak> {
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
    fn h0001_info_ack(&mut self, data: h0001::Ack<HSUB1>) -> Result<(), CommandError> {
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

    fn h0002_test_cmd(&mut self, data: h0002::Cmd<H>) -> Result<h0002::Ack<H>, h0002::Nak> {
        // Use first payload byte to lookup test entry
        // Then validate length
        let entry = &H0002ENTRIES[data.data[0] as usize];
        if entry.len == data.data.len() {
            Ok(h0002::Ack { data: data.data })
        } else {
            Err(h0002::Nak {})
        }
    }
    fn h0002_test_nacmd(&mut self, data: h0002::Cmd<H>) -> Result<(), CommandError> {
        // Use first payload byte to lookup test entry
        // Then validate length
        let entry = &H0002ENTRIES[data.data[0] as usize];
        if entry.len == data.data.len() {
            Ok(())
        } else {
            Err(CommandError::TestFailure)
        }
    }
    fn h0002_test_ack(&mut self, data: h0002::Ack<H>) -> Result<(), CommandError> {
        // Use first payload byte to lookup test entry
        // Then validate length
        let entry = &H0002ENTRIES[data.data[0] as usize];
        if entry.len == data.data.len() {
            Ok(())
        } else {
            Err(CommandError::TestFailure)
        }
    }

    fn h0016_flashmode_cmd(&mut self, _data: h0016::Cmd) -> Result<h0016::Ack, h0016::Nak> {
        Ok(h0016::Ack { scancode: 15 })
    }
    fn h0016_flashmode_ack(&mut self, data: h0016::Ack) -> Result<(), CommandError> {
        if data.scancode == 15 {
            Ok(())
        } else {
            Err(CommandError::TestFailure)
        }
    }

    fn h0017_unicodetext_cmd(&mut self, data: h0017::Cmd<H>) -> Result<h0017::Ack, h0017::Nak> {
        if data.string == "My UTF-8 string" {
            Ok(h0017::Ack {})
        } else {
            Err(h0017::Nak {})
        }
    }
    fn h0017_unicodetext_nacmd(&mut self, data: h0017::Cmd<H>) -> Result<(), CommandError> {
        if data.string == "My UTF-8 na string" {
            Ok(())
        } else {
            Err(CommandError::TestFailure)
        }
    }
    fn h0017_unicodetext_ack(&mut self, _data: h0017::Ack) -> Result<(), CommandError> {
        Ok(())
    }

    fn h0018_unicodestate_cmd(&mut self, data: h0018::Cmd<H>) -> Result<h0018::Ack, h0018::Nak> {
        if data.symbols == "ABC" {
            Ok(h0018::Ack {})
        } else {
            Err(h0018::Nak {})
        }
    }
    fn h0018_unicodestate_nacmd(&mut self, data: h0018::Cmd<H>) -> Result<(), CommandError> {
        if data.symbols == "DEF" {
            Ok(())
        } else {
            Err(CommandError::TestFailure)
        }
    }
    fn h0018_unicodestate_ack(&mut self, _data: h0018::Ack) -> Result<(), CommandError> {
        Ok(())
    }

    fn h0021_pixelsetting_cmd(&mut self, data: h0021::Cmd) -> Result<h0021::Ack, h0021::Nak> {
        if data.command != h0021::Command::InvalidCommand {
            return Err(h0021::Nak {});
        }
        if unsafe { data.argument.raw } == 5 {
            Ok(h0021::Ack {})
        } else {
            Err(h0021::Nak {})
        }
    }
    fn h0021_pixelsetting_nacmd(&mut self, data: h0021::Cmd) -> Result<(), CommandError> {
        if data.command != h0021::Command::InvalidCommand {
            return Err(CommandError::TestFailure);
        }
        if unsafe { data.argument.raw } == 5 {
            Ok(())
        } else {
            Err(CommandError::TestFailure)
        }
    }
    fn h0021_pixelsetting_ack(&mut self, _data: h0021::Ack) -> Result<(), CommandError> {
        Ok(())
    }

    fn h0026_directset_cmd(&mut self, data: h0026::Cmd<HSUB2>) -> Result<h0026::Ack, h0026::Nak> {
        if data.start_address != 5 {
            return Err(h0026::Nak {});
        }
        if data.data.len() == 3 {
            Ok(h0026::Ack {})
        } else {
            Err(h0026::Nak {})
        }
    }
    fn h0026_directset_nacmd(&mut self, data: h0026::Cmd<HSUB2>) -> Result<(), CommandError> {
        if data.start_address != 5 {
            return Err(CommandError::TestFailure);
        }
        if data.data.len() == 3 {
            Ok(())
        } else {
            Err(CommandError::TestFailure)
        }
    }
    fn h0026_directset_ack(&mut self, _data: h0026::Ack) -> Result<(), CommandError> {
        Ok(())
    }

    fn h001a_sleepmode_cmd(&mut self, _data: h001a::Cmd) -> Result<h001a::Ack, h001a::Nak> {
        Ok(h001a::Ack {})
    }
    fn h001a_sleepmode_ack(&mut self, _data: h001a::Ack) -> Result<(), CommandError> {
        Ok(())
    }

    fn h0030_openurl_cmd(&mut self, data: h0030::Cmd<H>) -> Result<h0030::Ack, h0030::Nak> {
        if data.url == "https://input.club" {
            Ok(h0030::Ack {})
        } else {
            Err(h0030::Nak {})
        }
    }
    fn h0030_openurl_ack(&mut self, _data: h0030::Ack) -> Result<(), CommandError> {
        Ok(())
    }

    fn h0031_terminalcmd_cmd(&mut self, data: h0031::Cmd<H>) -> Result<h0031::Ack, h0031::Nak> {
        if data.command == "terminal command string\n\r" {
            Ok(h0031::Ack {})
        } else {
            Err(h0031::Nak {})
        }
    }
    fn h0031_terminalcmd_nacmd(&mut self, data: h0031::Cmd<H>) -> Result<(), CommandError> {
        if data.command == "na terminal command string\n\r" {
            Ok(())
        } else {
            Err(CommandError::TestFailure)
        }
    }
    fn h0031_terminalcmd_ack(&mut self, _data: h0031::Ack) -> Result<(), CommandError> {
        Ok(())
    }

    fn h0034_terminalout_cmd(&mut self, data: h0034::Cmd<H>) -> Result<h0034::Ack, h0034::Nak> {
        if data.output == "terminal output string\n\r\t" {
            Ok(h0034::Ack {})
        } else {
            Err(h0034::Nak {})
        }
    }
    fn h0034_terminalout_nacmd(&mut self, data: h0034::Cmd<H>) -> Result<(), CommandError> {
        if data.output == "terminal na output string\n\r\t" {
            Ok(())
        } else {
            Err(CommandError::TestFailure)
        }
    }
    fn h0034_terminalout_ack(&mut self, _data: h0034::Ack) -> Result<(), CommandError> {
        Ok(())
    }

    fn h0050_manufacturing_cmd(&mut self, data: h0050::Cmd) -> Result<h0050::Ack, h0050::Nak> {
        if data.command == h0050::Command::TestCommand && unsafe { data.argument.raw == 0 } {
            Ok(h0050::Ack {})
        } else {
            Err(h0050::Nak {})
        }
    }
    fn h0050_manufacturing_ack(&mut self, _data: h0050::Ack) -> Result<(), CommandError> {
        Ok(())
    }
    fn h0050_manufacturing_nak(&mut self, _data: h0050::Nak) -> Result<(), CommandError> {
        Err(CommandError::TestFailure)
    }

    fn h0051_manufacturingres_cmd(
        &mut self,
        data: h0051::Cmd<HSUB4>,
    ) -> Result<h0051::Ack, h0051::Nak> {
        if data.command == h0051::Command::TestCommand && unsafe { data.argument.raw == 0 } {
            Ok(h0051::Ack {})
        } else {
            Err(h0051::Nak {})
        }
    }
    fn h0051_manufacturingres_ack(&mut self, _data: h0051::Ack) -> Result<(), CommandError> {
        Ok(())
    }
    fn h0051_manufacturingres_nak(&mut self, _data: h0051::Nak) -> Result<(), CommandError> {
        Err(CommandError::TestFailure)
    }
}

// ----- Tests -----

#[test]
fn h0000_supported_ids_test() {
    setup_logging_lite().ok();

    // Build list of supported ids
    let ids = [
        HidIoCommandId::SupportedIds,
        HidIoCommandId::GetInfo,
        HidIoCommandId::TestPacket,
    ];

    // Setup command interface
    let mut intf = CommandInterface::<8, 8, 64, 100, 99, 98, 96, 110, 3>::new(&ids).unwrap();

    // Send command
    let send = intf.h0000_supported_ids(h0000::Cmd {});
    assert!(send.is_ok(), "h0000_supported_ids => {:?}", send);

    // Flush tx->rx
    // Process rx buffer
    let process = intf.process_rx();
    assert!(process.is_ok(), "process_rx1 => {:?}", process);

    // Flush tx->rx
    // Process rx buffer
    let process = intf.process_rx();
    assert!(process.is_ok(), "process_rx2 => {:?}", process);
}

// Build test entries
#[derive(Debug)]
struct H0001TestEntry<'a> {
    property: h0001::Property,
    os: h0001::OsType,
    number: u16,
    string: &'a str,
}
const H0001ENTRIES: [H0001TestEntry; 13] = [
    H0001TestEntry {
        property: h0001::Property::MajorVersion,
        os: h0001::OsType::Unknown,
        number: 12,
        string: "",
    },
    H0001TestEntry {
        property: h0001::Property::MinorVersion,
        os: h0001::OsType::Unknown,
        number: 34,
        string: "",
    },
    H0001TestEntry {
        property: h0001::Property::PatchVersion,
        os: h0001::OsType::Unknown,
        number: 79,
        string: "",
    },
    H0001TestEntry {
        property: h0001::Property::DeviceName,
        os: h0001::OsType::Unknown,
        number: 0,
        string: "My Device",
    },
    H0001TestEntry {
        property: h0001::Property::DeviceSerialNumber,
        os: h0001::OsType::Unknown,
        number: 0,
        string: "1234567890 - 0987654321",
    },
    H0001TestEntry {
        property: h0001::Property::DeviceVersion,
        os: h0001::OsType::Unknown,
        number: 0,
        string: "v9001",
    },
    H0001TestEntry {
        property: h0001::Property::DeviceMcu,
        os: h0001::OsType::Unknown,
        number: 0,
        string: "someMCUname",
    },
    H0001TestEntry {
        property: h0001::Property::FirmwareName,
        os: h0001::OsType::Unknown,
        number: 0,
        string: "SpecialDeviceFirmware",
    },
    H0001TestEntry {
        property: h0001::Property::FirmwareVersion,
        os: h0001::OsType::Unknown,
        number: 0,
        string: "v9999",
    },
    H0001TestEntry {
        property: h0001::Property::DeviceVendor,
        os: h0001::OsType::Unknown,
        number: 0,
        string: "HID-IO",
    },
    H0001TestEntry {
        property: h0001::Property::OsType,
        os: h0001::OsType::Linux,
        number: 0,
        string: "",
    },
    H0001TestEntry {
        property: h0001::Property::OsVersion,
        os: h0001::OsType::Unknown,
        number: 0,
        string: "Special Linux Version",
    },
    H0001TestEntry {
        property: h0001::Property::HostSoftwareName,
        os: h0001::OsType::Unknown,
        number: 0,
        string: "HID-IO Core Unit Test",
    },
];

#[test]
fn h0001_info() {
    setup_logging_lite().ok();

    // Build list of supported ids
    let ids = [HidIoCommandId::SupportedIds, HidIoCommandId::GetInfo];

    // Setup command interface
    let mut intf = CommandInterface::<8, 8, 64, 100, 99, 98, 96, 110, 2>::new(&ids).unwrap();

    // Process each of the test entries
    for entry in &H0001ENTRIES {
        // Send command
        let send = intf.h0001_info(h0001::Cmd {
            property: entry.property,
        });
        assert!(send.is_ok(), "h0001_info {:?} => {:?}", entry, send);

        // Flush tx->rx
        // Process rx buffer
        let process = intf.process_rx();
        assert!(process.is_ok(), "process_rx1 {:?} => {:?}", entry, process);

        // Flush tx->rx
        // Process rx buffer
        let process = intf.process_rx();
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
fn h0002_test() {
    setup_logging_lite().ok();

    // Build list of supported ids
    let ids = [
        HidIoCommandId::SupportedIds,
        HidIoCommandId::GetInfo,
        HidIoCommandId::TestPacket,
    ];

    // Setup command interface
    let mut intf = CommandInterface::<8, 8, 64, 150, 149, 148, 146, 165, 3>::new(&ids).unwrap();

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
        // Process rx buffer
        let process = intf.process_rx();
        assert!(process.is_ok(), "process_rx1 {:?} => {:?}", entry, process);

        // Flush tx->rx
        // Process rx buffer
        let process = intf.process_rx();
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
        // Process rx buffer
        let process = intf.process_rx();
        assert!(process.is_ok(), "process_rx3 {:?} => {:?}", entry, process);
    }
}

#[test]
fn h0002_invalid() {
    setup_logging_lite().ok();

    // Build list of supported ids
    let ids = [HidIoCommandId::SupportedIds, HidIoCommandId::GetInfo];

    // Setup command interface
    let mut intf = CommandInterface::<8, 8, 64, 150, 149, 148, 146, 165, 2>::new(&ids).unwrap();

    // Send command
    let cmd = h0002::Cmd { data: Vec::new() };
    let send = intf.h0002_test(cmd, false);
    assert!(send.is_ok(), "h0002_invalid => {:?}", send);

    // Flush tx->rx
    // Process rx buffer (look for error)
    let process = intf.process_rx();
    assert!(process.is_err(), "process_rx1 => {:?}", process);

    // Cleanup after failure
    intf.rx_packetbuf.clear();

    // Send NA command
    let cmd = h0002::Cmd { data: Vec::new() };
    let send = intf.h0002_test(cmd, true);
    assert!(send.is_ok(), "h0002_invalid(na) => {:?}", send);

    // Flush tx->rx
    // Process rx buffer
    let process = intf.process_rx();
    assert!(process.is_err(), "process_rx2 => {:?}", process);
}

#[test]
fn h0016_flashmode() {
    setup_logging_lite().ok();

    // Build list of supported ids
    let ids = [HidIoCommandId::FlashMode];

    // Setup command interface
    let mut intf = CommandInterface::<8, 8, 64, 150, 149, 148, 146, 165, 1>::new(&ids).unwrap();

    // Send command
    let cmd = h0016::Cmd {};
    let send = intf.h0016_flashmode(cmd);
    assert!(send.is_ok(), "h0016_flashmode => {:?}", send);

    // Flush tx->rx
    // Process rx buffer
    let process = intf.process_rx();
    assert!(process.is_ok(), "process_rx1 => {:?}", process);

    // Flush tx->rx
    // Process rx buffer
    let process = intf.process_rx();
    assert!(process.is_ok(), "process_rx2 => {:?}", process);
}

#[test]
fn h0017_unicodetext() {
    setup_logging_lite().ok();

    // Build list of supported ids
    let ids = [HidIoCommandId::UnicodeText];

    // Setup command interface
    let mut intf = CommandInterface::<8, 8, 64, 150, 149, 148, 146, 165, 1>::new(&ids).unwrap();

    // Normal data packet
    // Send command
    let cmd = h0017::Cmd {
        string: String::from("My UTF-8 string"),
    };
    let send = intf.h0017_unicodetext(cmd.clone(), false);
    assert!(send.is_ok(), "h0017_unicodetext {:?} => {:?}", cmd, send);

    // Flush tx->rx
    // Process rx buffer
    let process = intf.process_rx();
    assert!(process.is_ok(), "process_rx1 {:?} => {:?}", cmd, process);

    // Flush tx->rx
    // Process rx buffer
    let process = intf.process_rx();
    assert!(process.is_ok(), "process_rx2 {:?} => {:?}", cmd, process);

    // NA (no-ack) data packets
    // Send command
    let cmd = h0017::Cmd {
        string: String::from("My UTF-8 na string"),
    };
    let send = intf.h0017_unicodetext(cmd.clone(), true);
    assert!(
        send.is_ok(),
        "h0017_unicodetext(na) {:?} => {:?}",
        cmd,
        send
    );

    // Flush tx->rx
    // Process rx buffer
    let process = intf.process_rx();
    assert!(process.is_ok(), "process_rx3 {:?} => {:?}", cmd, process);
}

#[test]
fn h0018_unicodestate() {
    setup_logging_lite().ok();

    // Build list of supported ids
    let ids = [HidIoCommandId::UnicodeState];

    // Setup command interface
    let mut intf = CommandInterface::<8, 8, 64, 150, 149, 148, 146, 165, 1>::new(&ids).unwrap();

    // Normal data packet
    // Send command
    let cmd = h0018::Cmd {
        symbols: String::from("ABC"),
    };
    let send = intf.h0018_unicodestate(cmd.clone(), false);
    assert!(send.is_ok(), "h0018_unicodestate {:?} => {:?}", cmd, send);

    // Flush tx->rx
    // Process rx buffer
    let process = intf.process_rx();
    assert!(process.is_ok(), "process_rx1 {:?} => {:?}", cmd, process);

    // Flush tx->rx
    // Process rx buffer
    let process = intf.process_rx();
    assert!(process.is_ok(), "process_rx2 {:?} => {:?}", cmd, process);

    // NA (no-ack) data packets
    // Send command
    let cmd = h0018::Cmd {
        symbols: String::from("DEF"),
    };
    let send = intf.h0018_unicodestate(cmd.clone(), true);
    assert!(
        send.is_ok(),
        "h0018_unicodestate(na) {:?} => {:?}",
        cmd,
        send
    );

    // Flush tx->rx
    // Process rx buffer
    let process = intf.process_rx();
    assert!(process.is_ok(), "process_rx3 {:?} => {:?}", cmd, process);
}

#[test]
fn h001a_sleepmode() {
    setup_logging_lite().ok();

    // Build list of supported ids
    let ids = [HidIoCommandId::SleepMode];

    // Setup command interface
    let mut intf = CommandInterface::<8, 8, 64, 150, 149, 148, 146, 165, 1>::new(&ids).unwrap();

    // Send command
    let cmd = h001a::Cmd {};
    let send = intf.h001a_sleepmode(cmd);
    assert!(send.is_ok(), "h001a_sleepmode => {:?}", send);

    // Flush tx->rx
    // Process rx buffer
    let process = intf.process_rx();
    assert!(process.is_ok(), "process_rx1 => {:?}", process);

    // Flush tx->rx
    // Process rx buffer
    let process = intf.process_rx();
    assert!(process.is_ok(), "process_rx2 => {:?}", process);
}

#[test]
fn h0021_pixelsetting() {
    setup_logging_lite().ok();

    // Build list of supported ids
    let ids = [HidIoCommandId::PixelSetting];

    // Setup command interface
    let mut intf = CommandInterface::<8, 8, 64, 150, 149, 148, 146, 165, 1>::new(&ids).unwrap();

    // Send command
    let cmd = h0021::Cmd {
        command: h0021::Command::InvalidCommand,
        argument: h0021::Argument { raw: 5 },
    };
    let send = intf.h0021_pixelsetting(cmd.clone(), true);
    assert!(send.is_ok(), "h0026_directset(na) => {:?}", send);

    let send = intf.h0021_pixelsetting(cmd, true);
    assert!(send.is_ok(), "h0026_directset(a) => {:?}", send);

    // Flush tx->rx
    // Process rx buffer
    let process = intf.process_rx();
    assert!(process.is_ok(), "process_rx1 => {:?}", process);
}

#[test]
fn h0026_directset() {
    setup_logging_lite().ok();

    // Build list of supported ids
    let ids = [HidIoCommandId::DirectSet];

    // Setup command interface
    let mut intf = CommandInterface::<8, 8, 64, 150, 149, 148, 146, 165, 1>::new(&ids).unwrap();

    // Send command
    let cmd = h0026::Cmd {
        start_address: 5,
        data: Vec::from_slice(&[1, 2, 3]).unwrap(),
    };
    let send = intf.h0026_directset(cmd.clone(), true);
    assert!(send.is_ok(), "h0026_directset(na) => {:?}", send);

    let send = intf.h0026_directset(cmd, true);
    assert!(send.is_ok(), "h0026_directset(a) => {:?}", send);

    // Flush tx->rx
    // Process rx buffer
    let process = intf.process_rx();
    assert!(process.is_ok(), "process_rx1 => {:?}", process);
}

#[test]
fn h0030_openurl() {
    setup_logging_lite().ok();

    // Build list of supported ids
    let ids = [HidIoCommandId::OpenUrl];

    // Setup command interface
    let mut intf = CommandInterface::<8, 8, 64, 150, 149, 148, 146, 165, 1>::new(&ids).unwrap();

    // Normal data packet
    // Send command
    let cmd = h0030::Cmd {
        url: String::from("https://input.club"),
    };
    let send = intf.h0030_openurl(cmd.clone());
    assert!(send.is_ok(), "h0030_openurl {:?} => {:?}", cmd, send);

    // Flush tx->rx
    // Process rx buffer
    let process = intf.process_rx();
    assert!(process.is_ok(), "process_rx1 {:?} => {:?}", cmd, process);

    // Flush tx->rx
    // Process rx buffer
    let process = intf.process_rx();
    assert!(process.is_ok(), "process_rx2 {:?} => {:?}", cmd, process);
}

#[test]
fn h0031_terminalcmd() {
    setup_logging_lite().ok();

    // Build list of supported ids
    let ids = [HidIoCommandId::TerminalCmd];

    // Setup command interface
    let mut intf = CommandInterface::<8, 8, 64, 150, 149, 148, 146, 165, 1>::new(&ids).unwrap();

    // Normal data packet
    // Send command
    let cmd = h0031::Cmd {
        command: String::from("terminal command string\n\r"),
    };
    let send = intf.h0031_terminalcmd(cmd.clone(), false);
    assert!(send.is_ok(), "h0031_terminalcmd {:?} => {:?}", cmd, send);

    // Flush tx->rx
    // Process rx buffer
    let process = intf.process_rx();
    assert!(process.is_ok(), "process_rx1 {:?} => {:?}", cmd, process);

    // Flush tx->rx
    // Process rx buffer
    let process = intf.process_rx();
    assert!(process.is_ok(), "process_rx2 {:?} => {:?}", cmd, process);

    // NA (no-ack) data packets
    // Send command
    let cmd = h0031::Cmd {
        command: String::from("na terminal command string\n\r"),
    };
    let send = intf.h0031_terminalcmd(cmd.clone(), true);
    assert!(
        send.is_ok(),
        "h0031_terminalcmd(na) {:?} => {:?}",
        cmd,
        send
    );

    // Flush tx->rx
    // Process rx buffer
    let process = intf.process_rx();
    assert!(process.is_ok(), "process_rx3 {:?} => {:?}", cmd, process);
}

#[test]
fn h0034_terminalout() {
    setup_logging_lite().ok();

    // Build list of supported ids
    let ids = [HidIoCommandId::TerminalOut];

    // Setup command interface
    let mut intf = CommandInterface::<8, 8, 64, 150, 149, 148, 146, 165, 1>::new(&ids).unwrap();

    // Normal data packet
    // Send command
    let cmd = h0034::Cmd {
        output: String::from("terminal output string\n\r\t"),
    };
    let send = intf.h0034_terminalout(cmd.clone(), false);
    assert!(send.is_ok(), "h0034_terminalout {:?} => {:?}", cmd, send);

    // Flush tx->rx
    // Process rx buffer
    let process = intf.process_rx();
    assert!(process.is_ok(), "process_rx1 {:?} => {:?}", cmd, process);

    // Flush tx->rx
    // Process rx buffer
    let process = intf.process_rx();
    assert!(process.is_ok(), "process_rx2 {:?} => {:?}", cmd, process);

    // NA (no-ack) data packets
    // Send command
    let cmd = h0034::Cmd {
        output: String::from("terminal na output string\n\r\t"),
    };
    let send = intf.h0034_terminalout(cmd.clone(), true);
    assert!(
        send.is_ok(),
        "h0034_terminalout(na) {:?} => {:?}",
        cmd,
        send
    );

    // Flush tx->rx
    // Process rx buffer
    let process = intf.process_rx();
    assert!(process.is_ok(), "process_rx3 {:?} => {:?}", cmd, process);
}

#[test]
fn h0050_manufacturing() {
    setup_logging_lite().ok();

    // Build list of supported ids
    let ids = [HidIoCommandId::ManufacturingTest];

    // Setup command interface
    let mut intf = CommandInterface::<8, 8, 64, 150, 149, 148, 146, 165, 1>::new(&ids).unwrap();

    // Send valid command (expect ack)
    let cmd = h0050::Cmd {
        command: h0050::Command::TestCommand,
        argument: h0050::Argument { raw: 0 },
    };
    let send = intf.h0050_manufacturing(cmd);
    assert!(send.is_ok(), "h0050_manufacturing(ack) => {:?}", send);

    // Flush tx->rx
    // Process rx buffer
    let process = intf.process_rx();
    assert!(process.is_ok(), "process_rx1 => {:?}", process);

    // Flush tx->rx
    // Process rx buffer
    let process = intf.process_rx();
    assert!(process.is_ok(), "process_rx2 => {:?}", process);

    // Send invalid command (expect nak)
    let cmd = h0050::Cmd {
        command: h0050::Command::InvalidCommand,
        argument: h0050::Argument { raw: 5 },
    };
    let send = intf.h0050_manufacturing(cmd);
    assert!(send.is_ok(), "h0050_manufacturing(nak) => {:?}", send);

    // Flush tx->rx
    // Process rx buffer
    let process = intf.process_rx();
    assert!(process.is_ok(), "process_rx3 => {:?}", process);

    // Flush tx->rx
    // Process rx buffer
    let process = intf.process_rx();
    assert!(process.is_err(), "process_rx4 => {:?}", process);
}

#[test]
fn h0051_manufacturing() {
    setup_logging_lite().ok();

    // Build list of supported ids
    let ids = [HidIoCommandId::ManufacturingResult];

    // Setup command interface
    let mut intf = CommandInterface::<8, 8, 64, 150, 149, 148, 146, 165, 1>::new(&ids).unwrap();

    // Send valid command (expect ack)
    let cmd = h0051::Cmd {
        command: h0051::Command::TestCommand,
        argument: h0051::Argument { raw: 0 },
        data: Vec::new(),
    };
    let send = intf.h0051_manufacturingres(cmd);
    assert!(send.is_ok(), "h0051_manufacturing(ack) => {:?}", send);

    // Flush tx->rx
    // Process rx buffer
    let process = intf.process_rx();
    assert!(process.is_ok(), "process_rx1 => {:?}", process);

    // Flush tx->rx
    // Process rx buffer
    let process = intf.process_rx();
    assert!(process.is_ok(), "process_rx2 => {:?}", process);
}
