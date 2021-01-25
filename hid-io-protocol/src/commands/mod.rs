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

// ----- Crates -----

use super::*;
use core::convert::{TryFrom, TryInto};
use heapless::{String, Vec};

pub use core::ops::Sub;
pub use heapless::consts::B1;
pub use heapless::ArrayLength;
pub use typenum::Sub1;

// ----- Modules -----

mod test;

// ----- Macros -----

// ----- Enumerations -----

#[derive(Debug)]
pub enum CommandError {
    BufferInUse,
    BufferNotReady,
    DataVecNoData,
    DataVecTooSmall,
    IdNotImplemented(HidIoCommandID, HidIoPacketType),
    IdNotMatched(HidIoCommandID),
    IdNotSupported(HidIoCommandID),
    IdVecTooSmall,
    InvalidId(u32),
    InvalidPacketBufferType(HidIoPacketType),
    InvalidProperty8(u8),
    InvalidRxMessage(HidIoPacketType),
    InvalidUtf8(core::str::Utf8Error),
    PacketDecodeError(HidIoParseError),
    RxFailed,
    RxTimeout,
    RxTooManySyncs,
    SerializationFailed(HidIoParseError),
    SerializationVecTooSmall,
    TestFailure,
    TxBufferSendFailed,
    TxBufferVecTooSmall,
    TxNoActiveReceivers,
}

// ----- Command Structs -----

/// Supported Ids
pub mod h0000 {
    use super::super::HidIoCommandID;
    use heapless::{ArrayLength, Vec};

    #[derive(Clone, Debug)]
    pub struct Cmd {}

    #[derive(Clone, Debug)]
    pub struct Ack<ID: ArrayLength<HidIoCommandID>> {
        pub ids: Vec<HidIoCommandID, ID>,
    }

    #[derive(Clone, Debug)]
    pub struct Nak {}
}

/// Info Query
pub mod h0001 {
    use heapless::{ArrayLength, String};
    use num_enum::TryFromPrimitive;

    #[repr(u8)]
    #[derive(PartialEq, Clone, Copy, Debug, TryFromPrimitive)]
    pub enum Property {
        Unknown = 0x00,
        MajorVersion = 0x01,
        MinorVersion = 0x02,
        PatchVersion = 0x03,
        DeviceName = 0x04,
        DeviceSerialNumber = 0x05,
        DeviceVersion = 0x06,
        DeviceMCU = 0x07,
        FirmwareName = 0x08,
        FirmwareVersion = 0x09,
        DeviceVendor = 0x0A,
        OsType = 0x0B,
        OsVersion = 0x0C,
        HostSoftwareName = 0x0D,
    }

    #[repr(u8)]
    #[derive(PartialEq, Clone, Copy, Debug, TryFromPrimitive)]
    pub enum OSType {
        Unknown = 0x00,
        Windows = 0x01,
        Linux = 0x02,
        Android = 0x03,
        MacOS = 0x04,
        IOS = 0x05,
        ChromeOS = 0x06,
    }

    #[derive(Clone, Debug)]
    pub struct Cmd {
        pub property: Property,
    }

    #[derive(Clone, Debug)]
    pub struct Ack<S: ArrayLength<u8>> {
        pub property: Property,

        /// OS Type field
        pub os: OSType,

        /// Number is set when the given property specifies a number
        pub number: u16,

        /// String is set when the given property specifies a string
        /// Should be 1 byte less than the max hidio data buffer size
        pub string: String<S>,
    }

    #[derive(Clone, Debug)]
    pub struct Nak {
        pub property: Property,
    }
}

/// Test Message
pub mod h0002 {
    use heapless::{ArrayLength, Vec};

    #[derive(Clone, Debug)]
    pub struct Cmd<D: ArrayLength<u8>> {
        pub data: Vec<u8, D>,
    }

    #[derive(Clone, Debug)]
    pub struct Ack<D: ArrayLength<u8>> {
        pub data: Vec<u8, D>,
    }

    #[derive(Clone, Debug)]
    pub struct Nak {}
}

/// Reset HID-IO
pub mod h0003 {
    #[derive(Clone, Debug)]
    pub struct Cmd {}

    #[derive(Clone, Debug)]
    pub struct Ack {}

    #[derive(Clone, Debug)]
    pub struct Nak {}
}

/// Get Properties
pub mod h0010 {
    use heapless::{ArrayLength, String, Vec};
    use num_enum::TryFromPrimitive;

    #[repr(u8)]
    #[derive(PartialEq, Clone, Copy, Debug, TryFromPrimitive)]
    pub enum Command {
        ListFields = 0x00,
        GetFieldName = 0x01,
        GetFieldValue = 0x02,
        Unknown = 0xFF,
    }

    #[derive(Clone, Debug)]
    pub struct Cmd {
        pub command: Command,

        /// 8-bit field id
        /// Ignored by ListFields
        pub field: u8,
    }

    #[derive(Clone, Debug)]
    pub struct Ack<S: ArrayLength<u8>> {
        pub command: Command,

        /// 8-bit field id
        /// Ignored by ListFields
        pub field: u8,

        /// List of field ids
        pub field_list: Vec<u8, S>,

        /// String payload
        pub string: String<S>,
    }

    #[derive(Clone, Debug)]
    pub struct Nak {
        pub command: Command,

        /// 8-bit field id
        /// Ignored by ListFields
        pub field: u8,
    }
}

/// USB Key State
/// TODO
pub mod h0011 {
    pub struct Cmd {}
    pub struct Ack {}
    pub struct Nak {}
}

/// Keyboard Layout
/// TODO
pub mod h0012 {
    pub struct Cmd {}
    pub struct Ack {}
    pub struct Nak {}
}

/// Button Layout
/// TODO
pub mod h0013 {
    pub struct Cmd {}
    pub struct Ack {}
    pub struct Nak {}
}

/// Keycap Types
/// TODO
pub mod h0014 {
    pub struct Cmd {}
    pub struct Ack {}
    pub struct Nak {}
}

/// LED Layout
/// TODO
pub mod h0015 {
    pub struct Cmd {}
    pub struct Ack {}
    pub struct Nak {}
}

/// Flash Mode
pub mod h0016 {
    use num_enum::TryFromPrimitive;

    #[repr(u8)]
    #[derive(PartialEq, Clone, Copy, Debug, TryFromPrimitive)]
    pub enum Error {
        NotSupported = 0x00,
        Disabled = 0x01,
    }

    #[derive(Clone, Debug)]
    pub struct Cmd {}

    #[derive(Clone, Debug)]
    pub struct Ack {
        pub scancode: u16,
    }

    #[derive(Clone, Debug)]
    pub struct Nak {
        pub error: Error,
    }
}

/// UTF-8 Character Stream
pub mod h0017 {
    use heapless::{ArrayLength, String};

    #[derive(Clone, Debug)]
    pub struct Cmd<S: ArrayLength<u8>> {
        pub string: String<S>,
    }

    #[derive(Clone, Debug)]
    pub struct Ack {}

    #[derive(Clone, Debug)]
    pub struct Nak {}
}

/// UTF-8 State
pub mod h0018 {
    use heapless::{ArrayLength, String};

    #[derive(Clone, Debug)]
    pub struct Cmd<S: ArrayLength<u8>> {
        pub symbols: String<S>,
    }

    #[derive(Clone, Debug)]
    pub struct Ack {}

    #[derive(Clone, Debug)]
    pub struct Nak {}
}

/// Trigger Host Macro
/// TODO
pub mod h0019 {
    pub struct Cmd {}
    pub struct Ack {}
    pub struct Nak {}
}

/// Sleep Mode
pub mod h001a {
    use num_enum::TryFromPrimitive;

    #[repr(u8)]
    #[derive(PartialEq, Clone, Copy, Debug, TryFromPrimitive)]
    pub enum Error {
        NotSupported = 0x00,
        Disabled = 0x01,
        NotReady = 0x02,
    }

    #[derive(Clone, Debug)]
    pub struct Cmd {}

    #[derive(Clone, Debug)]
    pub struct Ack {}

    #[derive(Clone, Debug)]
    pub struct Nak {
        pub error: Error,
    }
}

/// KLL Trigger State
/// TODO
pub mod h0020 {
    pub struct Cmd {}
    pub struct Ack {}
    pub struct Nak {}
}

/// Pixel Settings
/// TODO
pub mod h0021 {
    pub struct Cmd {}
    pub struct Ack {}
    pub struct Nak {}
}

/// Pixel Set (1ch, 8bit)
/// TODO
pub mod h0022 {
    pub struct Cmd {}
    pub struct Ack {}
    pub struct Nak {}
}

/// Pixel Set (3ch, 8bit)
/// TODO
pub mod h0023 {
    pub struct Cmd {}
    pub struct Ack {}
    pub struct Nak {}
}

/// Pixel Set (1ch, 16bit)
/// TODO
pub mod h0024 {
    pub struct Cmd {}
    pub struct Ack {}
    pub struct Nak {}
}

/// Pixel Set (3ch, 16bit)
/// TODO
pub mod h0025 {
    pub struct Cmd {}
    pub struct Ack {}
    pub struct Nak {}
}

/// Open URL
/// TODO
pub mod h0030 {
    pub struct Cmd {}
    pub struct Ack {}
    pub struct Nak {}
}

/// Terminal Command
pub mod h0031 {
    use heapless::{ArrayLength, String};

    #[derive(Clone, Debug)]
    pub struct Cmd<S: ArrayLength<u8>> {
        pub command: String<S>,
    }

    #[derive(Clone, Debug)]
    pub struct Ack {}

    #[derive(Clone, Debug)]
    pub struct Nak {}
}

/// Get OS Layout
/// TODO
pub mod h0032 {
    pub struct Cmd {}
    pub struct Ack {}
    pub struct Nak {}
}

/// Set OS Layout
/// TODO
pub mod h0033 {
    pub struct Cmd {}
    pub struct Ack {}
    pub struct Nak {}
}

/// Terminal Output
pub mod h0034 {
    use heapless::{ArrayLength, String};

    #[derive(Clone, Debug)]
    pub struct Cmd<S: ArrayLength<u8>> {
        pub output: String<S>,
    }

    #[derive(Clone, Debug)]
    pub struct Ack {}

    #[derive(Clone, Debug)]
    pub struct Nak {}
}

/// HID Keyboard State
/// TODO
pub mod h0040 {
    pub struct Cmd {}
    pub struct Ack {}
    pub struct Nak {}
}

/// HID Keyboard LED State
/// TODO
pub mod h0041 {
    pub struct Cmd {}
    pub struct Ack {}
    pub struct Nak {}
}

/// HID Mouse State
/// TODO
pub mod h0042 {
    pub struct Cmd {}
    pub struct Ack {}
    pub struct Nak {}
}

/// HID Joystick State
/// TODO
pub mod h0043 {
    pub struct Cmd {}
    pub struct Ack {}
    pub struct Nak {}
}

/// Manufacturing Test
pub mod h0050 {
    use heapless::{ArrayLength, Vec};

    #[derive(Clone, Debug)]
    pub struct Cmd {
        pub command: u16,
        pub argument: u16,
    }

    #[derive(Clone, Debug)]
    pub struct Ack<D: ArrayLength<u8>> {
        pub data: Vec<u8, D>,
    }

    #[derive(Clone, Debug)]
    pub struct Nak<D: ArrayLength<u8>> {
        pub data: Vec<u8, D>,
    }
}

// ----- Traits -----

/// HID-IO Command Interface
/// H - Max data payload length (HidIoPacketBuffer)
/// ID - Max number of HidIoCommandIDs
pub trait Commands<H: ArrayLength<u8>, ID: ArrayLength<HidIoCommandID> + ArrayLength<u8>>
where
    H: core::fmt::Debug + Sub<B1>,
{
    /// Given a HidIoPacketBuffer serialize (and resulting send bytes)
    fn tx_packetbuffer_send(&mut self, buf: &mut HidIoPacketBuffer<H>) -> Result<(), CommandError>;

    /// Check if id is valid for this interface
    /// (By default support all ids)
    fn supported_id(&self, _id: HidIoCommandID) -> bool {
        true
    }

    /// Default packet chunk
    /// (Usual chunk sizes are 63 or 64)
    fn default_packet_chunk(&self) -> u32 {
        64
    }

    /// Simple empty ack
    fn empty_ack(&mut self, id: HidIoCommandID) -> Result<(), CommandError> {
        // Build empty ACK
        self.tx_packetbuffer_send(&mut HidIoPacketBuffer {
            // Data packet
            ptype: HidIoPacketType::ACK,
            // Packet id
            id,
            // Detect max size
            max_len: self.default_packet_chunk(),
            // Ready to go
            done: true,
            // Use defaults for other fields
            ..Default::default()
        })
    }

    /// Simple empty nak
    fn empty_nak(&mut self, id: HidIoCommandID) -> Result<(), CommandError> {
        // Build empty NAK
        self.tx_packetbuffer_send(&mut HidIoPacketBuffer {
            // Data packet
            ptype: HidIoPacketType::NAK,
            // Packet id
            id,
            // Detect max size
            max_len: self.default_packet_chunk(),
            // Ready to go
            done: true,
            // Use defaults for other fields
            ..Default::default()
        })
    }

    /// Simple byte ack
    fn byte_ack(&mut self, id: HidIoCommandID, byte: u8) -> Result<(), CommandError> {
        // Build ACK
        self.tx_packetbuffer_send(&mut HidIoPacketBuffer {
            // Data packet
            ptype: HidIoPacketType::ACK,
            // Packet id
            id,
            // Detect max size
            max_len: self.default_packet_chunk(),
            // Byte payload
            data: Vec::from_slice(&[byte]).unwrap(),
            // Ready to go
            done: true,
        })
    }

    /// Simple byte nak
    fn byte_nak(&mut self, id: HidIoCommandID, byte: u8) -> Result<(), CommandError> {
        // Build NAK
        self.tx_packetbuffer_send(&mut HidIoPacketBuffer {
            // Data packet
            ptype: HidIoPacketType::NAK,
            // Packet id
            id,
            // Detect max size
            max_len: self.default_packet_chunk(),
            // Byte payload
            data: Vec::from_slice(&[byte]).unwrap(),
            // Ready to go
            done: true,
        })
    }

    /// Simple short ack (16-bit)
    fn short_ack(&mut self, id: HidIoCommandID, val: u16) -> Result<(), CommandError> {
        // Build ACK
        self.tx_packetbuffer_send(&mut HidIoPacketBuffer {
            // Data packet
            ptype: HidIoPacketType::ACK,
            // Packet id
            id,
            // Detect max size
            max_len: self.default_packet_chunk(),
            // Byte payload
            data: Vec::from_slice(&val.to_le_bytes()).unwrap(),
            // Ready to go
            done: true,
        })
    }

    /// Simple short nak (16-bit)
    fn short_nak(&mut self, id: HidIoCommandID, val: u16) -> Result<(), CommandError> {
        // Build NAK
        self.tx_packetbuffer_send(&mut HidIoPacketBuffer {
            // Data packet
            ptype: HidIoPacketType::NAK,
            // Packet id
            id,
            // Detect max size
            max_len: self.default_packet_chunk(),
            // Byte payload
            data: Vec::from_slice(&val.to_le_bytes()).unwrap(),
            // Ready to go
            done: true,
        })
    }

    /// Process specific packet types
    /// Handles matching to interface functions
    fn rx_message_handling(&mut self, buf: HidIoPacketBuffer<H>) -> Result<(), CommandError>
    where
        <H as Sub<B1>>::Output: ArrayLength<u8>,
    {
        // Make sure we're processing a supported id
        if !self.supported_id(buf.id) {
            return Err(CommandError::IdNotSupported(buf.id));
        }

        // Check for invalid packet types
        match buf.ptype {
            HidIoPacketType::Data | HidIoPacketType::NAData => {}
            HidIoPacketType::ACK => {}
            HidIoPacketType::NAK => {}
            _ => {
                return Err(CommandError::InvalidRxMessage(buf.ptype));
            }
        }

        // Match id
        match buf.id {
            HidIoCommandID::SupportedIDs => self.h0000_supported_ids_handler(buf),
            HidIoCommandID::GetInfo => self.h0001_info_handler(buf),
            HidIoCommandID::TestPacket => self.h0002_test_handler(buf),
            HidIoCommandID::ResetHidIo => self.h0003_resethidio_handler(buf),
            HidIoCommandID::FlashMode => self.h0016_flashmode_handler(buf),
            HidIoCommandID::UnicodeText => self.h0017_unicodetext_handler(buf),
            HidIoCommandID::UnicodeState => self.h0018_unicodestate_handler(buf),
            HidIoCommandID::SleepMode => self.h001a_sleepmode_handler(buf),
            HidIoCommandID::TerminalCmd => self.h0031_terminalcmd_handler(buf),
            HidIoCommandID::TerminalOut => self.h0034_terminalout_handler(buf),
            HidIoCommandID::ManufacturingTest => self.h0050_manufacturing_handler(buf),
            _ => Err(CommandError::IdNotMatched(buf.id)),
        }
    }

    fn h0000_supported_ids(&mut self, _data: h0000::Cmd) -> Result<(), CommandError> {
        self.tx_packetbuffer_send(&mut HidIoPacketBuffer {
            // Test packet id
            id: HidIoCommandID::SupportedIDs,
            // Detect max size
            max_len: self.default_packet_chunk(),
            // Ready to go
            done: true,
            // Use defaults for other fields
            ..Default::default()
        })
    }
    fn h0000_supported_ids_cmd(&mut self, _data: h0000::Cmd) -> Result<h0000::Ack<ID>, h0000::Nak> {
        Err(h0000::Nak {})
    }
    fn h0000_supported_ids_ack(&mut self, _data: h0000::Ack<ID>) -> Result<(), CommandError> {
        Err(CommandError::IdNotImplemented(
            HidIoCommandID::SupportedIDs,
            HidIoPacketType::ACK,
        ))
    }
    fn h0000_supported_ids_nak(&mut self, _data: h0000::Nak) -> Result<(), CommandError> {
        Err(CommandError::IdNotImplemented(
            HidIoCommandID::SupportedIDs,
            HidIoPacketType::NAK,
        ))
    }
    fn h0000_supported_ids_handler(
        &mut self,
        buf: HidIoPacketBuffer<H>,
    ) -> Result<(), CommandError> {
        // Handle packet type
        match buf.ptype {
            HidIoPacketType::Data => {
                match self.h0000_supported_ids_cmd(h0000::Cmd {}) {
                    Ok(ack) => {
                        // Build ACK
                        let mut buf = HidIoPacketBuffer {
                            // Data packet
                            ptype: HidIoPacketType::ACK,
                            // Packet id
                            id: buf.id,
                            // Detect max size
                            max_len: self.default_packet_chunk(),
                            // Ready to go
                            done: true,
                            // Use defaults for other fields
                            ..Default::default()
                        };

                        // Build list of ids
                        for id in ack.ids {
                            if buf
                                .data
                                .extend_from_slice(&(id as u16).to_le_bytes())
                                .is_err()
                            {
                                return Err(CommandError::IdVecTooSmall);
                            }
                        }
                        self.tx_packetbuffer_send(&mut buf)
                    }
                    Err(_nak) => self.empty_nak(buf.id),
                }
            }
            HidIoPacketType::NAData => Err(CommandError::InvalidPacketBufferType(buf.ptype)),
            HidIoPacketType::ACK => {
                // Retrieve list of ids
                let mut ids: Vec<HidIoCommandID, ID> = Vec::new();
                // Ids are always 16-bit le for this command
                let mut pos = 0;
                while pos <= buf.data.len() - 2 {
                    let slice = &buf.data[pos..pos + 2];
                    let idnum = u16::from_le_bytes(slice.try_into().unwrap()) as u32;
                    // Make sure this is a valid id
                    let id = match HidIoCommandID::try_from(idnum) {
                        Ok(id) => id,
                        Err(_) => {
                            return Err(CommandError::InvalidId(idnum));
                        }
                    };
                    // Attempt to push to id list
                    // NOTE: If the vector is not large enough
                    //       just truncate.
                    //       This command won't be called by devices
                    //       often.
                    // TODO: Add optional fields to request a range
                    if ids.push(id).is_err() {
                        break;
                    }
                    pos += 2;
                }
                self.h0000_supported_ids_ack(h0000::Ack { ids })
            }
            HidIoPacketType::NAK => self.h0000_supported_ids_nak(h0000::Nak {}),
            _ => Ok(()),
        }
    }

    fn h0001_info(&mut self, data: h0001::Cmd) -> Result<(), CommandError> {
        // Create appropriately sized buffer
        let mut buf = HidIoPacketBuffer {
            // Test packet id
            id: HidIoCommandID::GetInfo,
            // Detect max size
            max_len: self.default_packet_chunk(),
            // Ready to go
            done: true,
            // Use defaults for other fields
            ..Default::default()
        };

        // Encode property
        if buf.data.push(data.property as u8).is_err() {
            return Err(CommandError::DataVecTooSmall);
        }

        self.tx_packetbuffer_send(&mut buf)
    }
    fn h0001_info_cmd(&mut self, _data: h0001::Cmd) -> Result<h0001::Ack<Sub1<H>>, h0001::Nak>
    where
        <H as Sub<B1>>::Output: ArrayLength<u8>,
    {
        Err(h0001::Nak {
            property: h0001::Property::Unknown,
        })
    }
    fn h0001_info_ack(&mut self, _data: h0001::Ack<Sub1<H>>) -> Result<(), CommandError>
    where
        <H as Sub<B1>>::Output: ArrayLength<u8>,
    {
        Err(CommandError::IdNotImplemented(
            HidIoCommandID::GetInfo,
            HidIoPacketType::ACK,
        ))
    }
    fn h0001_info_nak(&mut self, _data: h0001::Nak) -> Result<(), CommandError> {
        Err(CommandError::IdNotImplemented(
            HidIoCommandID::GetInfo,
            HidIoPacketType::NAK,
        ))
    }
    fn h0001_info_handler(&mut self, buf: HidIoPacketBuffer<H>) -> Result<(), CommandError>
    where
        <H as Sub<B1>>::Output: ArrayLength<u8>,
    {
        // Handle packet type
        match buf.ptype {
            HidIoPacketType::Data => {
                if buf.data.len() < 1 {
                    return Err(CommandError::DataVecNoData);
                }
                // Attempt to read first byte
                let property = match h0001::Property::try_from(buf.data[0]) {
                    Ok(property) => property,
                    Err(_) => {
                        return Err(CommandError::InvalidProperty8(buf.data[0]));
                    }
                };
                match self.h0001_info_cmd(h0001::Cmd { property }) {
                    Ok(ack) => {
                        // Build ACK
                        let mut buf = HidIoPacketBuffer {
                            // Data packet
                            ptype: HidIoPacketType::ACK,
                            // Packet id
                            id: buf.id,
                            // Detect max size
                            max_len: self.default_packet_chunk(),
                            // Ready to go
                            done: true,
                            // Use defaults for other fields
                            ..Default::default()
                        };

                        // Set property
                        if buf.data.push(ack.property as u8).is_err() {
                            return Err(CommandError::DataVecTooSmall);
                        }

                        // Depending on the property set the rest
                        // of the data field
                        match property {
                            h0001::Property::Unknown => {}
                            // Handle 16-bit number type
                            h0001::Property::MajorVersion
                            | h0001::Property::MinorVersion
                            | h0001::Property::PatchVersion => {
                                // Convert to byte le bytes
                                for byte in &ack.number.to_le_bytes() {
                                    if buf.data.push(*byte).is_err() {
                                        return Err(CommandError::DataVecTooSmall);
                                    }
                                }
                            }
                            // Handle 8-bit os type
                            h0001::Property::OsType => {
                                if buf.data.push(ack.os as u8).is_err() {
                                    return Err(CommandError::DataVecTooSmall);
                                }
                            }
                            // Handle ascii values
                            _ => {
                                for byte in ack.string.into_bytes() {
                                    if buf.data.push(byte).is_err() {
                                        return Err(CommandError::DataVecTooSmall);
                                    }
                                }
                            }
                        }

                        self.tx_packetbuffer_send(&mut buf)
                    }
                    Err(_nak) => self.byte_nak(buf.id, property as u8),
                }
            }
            HidIoPacketType::NAData => Err(CommandError::InvalidPacketBufferType(buf.ptype)),
            HidIoPacketType::ACK => {
                if buf.data.len() < 1 {
                    return Err(CommandError::DataVecNoData);
                }
                // Attempt to read first byte
                let property = match h0001::Property::try_from(buf.data[0]) {
                    Ok(property) => property,
                    Err(_) => {
                        return Err(CommandError::InvalidProperty8(buf.data[0]));
                    }
                };

                // Setup ack struct
                let mut ack = h0001::Ack {
                    property,
                    os: h0001::OSType::Unknown,
                    number: 0,
                    string: String::new(),
                };

                // Depending on the property set the rest
                // of the ack fields
                match property {
                    h0001::Property::Unknown => {}
                    // Handle 16-bit number type
                    h0001::Property::MajorVersion
                    | h0001::Property::MinorVersion
                    | h0001::Property::PatchVersion => {
                        // Convert from le bytes
                        ack.number = u16::from_le_bytes(buf.data[1..3].try_into().unwrap());
                    }
                    // Handle 8-bit os type
                    h0001::Property::OsType => {
                        let typenum = buf.data[1];
                        ack.os = match h0001::OSType::try_from(typenum) {
                            Ok(ostype) => ostype,
                            Err(_) => {
                                return Err(CommandError::InvalidProperty8(typenum));
                            }
                        };
                    }
                    // Handle ascii values
                    _ => {
                        // NOTE: This is annoyingly inefficient?
                        let bytes: Vec<u8, Sub1<H>> = Vec::from_slice(&buf.data[1..]).unwrap();
                        let string = match String::from_utf8(bytes) {
                            Ok(string) => string,
                            Err(e) => {
                                return Err(CommandError::InvalidUtf8(e));
                            }
                        };
                        ack.string = string;
                    }
                }

                self.h0001_info_ack(ack)
            }
            HidIoPacketType::NAK => {
                if buf.data.len() < 1 {
                    return Err(CommandError::DataVecNoData);
                }
                // Attempt to read first byte
                let property = match h0001::Property::try_from(buf.data[0]) {
                    Ok(property) => property,
                    Err(_) => {
                        return Err(CommandError::InvalidProperty8(buf.data[0]));
                    }
                };
                self.h0001_info_nak(h0001::Nak { property })
            }
            _ => Ok(()),
        }
    }

    fn h0002_test(&mut self, data: h0002::Cmd<H>, na: bool) -> Result<(), CommandError> {
        // Create appropriately sized buffer
        let mut buf = HidIoPacketBuffer {
            // Test packet id
            id: HidIoCommandID::TestPacket,
            // Detect max size
            max_len: self.default_packet_chunk(),
            // Use defaults for other fields
            ..Default::default()
        };

        // Set NA (no-ack)
        if na {
            buf.ptype = HidIoPacketType::NAData;
        }

        // Build payload
        if !buf.append_payload(&data.data) {
            return Err(CommandError::DataVecTooSmall);
        }
        buf.done = true;

        self.tx_packetbuffer_send(&mut buf)
    }
    fn h0002_test_cmd(&mut self, _data: h0002::Cmd<H>) -> Result<h0002::Ack<H>, h0002::Nak> {
        Err(h0002::Nak {})
    }
    fn h0002_test_nacmd(&mut self, _data: h0002::Cmd<H>) -> Result<(), CommandError> {
        Err(CommandError::IdNotImplemented(
            HidIoCommandID::TestPacket,
            HidIoPacketType::NAData,
        ))
    }
    fn h0002_test_ack(&mut self, _data: h0002::Ack<H>) -> Result<(), CommandError> {
        Err(CommandError::IdNotImplemented(
            HidIoCommandID::TestPacket,
            HidIoPacketType::ACK,
        ))
    }
    fn h0002_test_nak(&mut self, _data: h0002::Nak) -> Result<(), CommandError> {
        Err(CommandError::IdNotImplemented(
            HidIoCommandID::TestPacket,
            HidIoPacketType::NAK,
        ))
    }
    fn h0002_test_handler(&mut self, buf: HidIoPacketBuffer<H>) -> Result<(), CommandError> {
        // Handle packet type
        match buf.ptype {
            HidIoPacketType::Data => {
                // Copy data into struct
                let cmd = h0002::Cmd::<H> {
                    data: match Vec::from_slice(&buf.data) {
                        Ok(data) => data,
                        Err(_) => {
                            return Err(CommandError::DataVecTooSmall);
                        }
                    },
                };

                match self.h0002_test_cmd(cmd) {
                    Ok(ack) => {
                        // Build ACK (max test data size)
                        let mut buf = HidIoPacketBuffer {
                            // Data packet
                            ptype: HidIoPacketType::ACK,
                            // Packet id
                            id: buf.id,
                            // Detect max size
                            max_len: self.default_packet_chunk(),
                            ..Default::default()
                        };

                        // Copy data into buffer
                        if !buf.append_payload(&ack.data) {
                            return Err(CommandError::DataVecTooSmall);
                        }
                        buf.done = true;
                        self.tx_packetbuffer_send(&mut buf)
                    }
                    Err(_nak) => self.empty_nak(buf.id),
                }
            }
            HidIoPacketType::NAData => {
                // Copy data into struct
                let cmd = h0002::Cmd::<H> {
                    data: match Vec::from_slice(&buf.data) {
                        Ok(data) => data,
                        Err(_) => {
                            return Err(CommandError::DataVecTooSmall);
                        }
                    },
                };

                self.h0002_test_nacmd(cmd)
            }
            HidIoPacketType::ACK => {
                // Copy data into struct
                let ack = h0002::Ack::<H> {
                    data: match Vec::from_slice(&buf.data) {
                        Ok(data) => data,
                        Err(_) => {
                            return Err(CommandError::DataVecTooSmall);
                        }
                    },
                };

                self.h0002_test_ack(ack)
            }
            HidIoPacketType::NAK => self.h0002_test_nak(h0002::Nak {}),
            _ => Ok(()),
        }
    }

    fn h0003_resethidio(&mut self, _data: h0003::Cmd) -> Result<(), CommandError> {
        self.tx_packetbuffer_send(&mut HidIoPacketBuffer {
            // Test packet id
            id: HidIoCommandID::ResetHidIo,
            // Detect max size
            max_len: self.default_packet_chunk(),
            // Ready
            done: true,
            // Use defaults for other fields
            ..Default::default()
        })
    }
    fn h0003_resethidio_cmd(&mut self, _data: h0003::Cmd) -> Result<h0003::Ack, h0003::Nak> {
        Err(h0003::Nak {})
    }
    fn h0003_resethidio_ack(&mut self, _data: h0003::Ack) -> Result<(), CommandError> {
        Err(CommandError::IdNotImplemented(
            HidIoCommandID::ResetHidIo,
            HidIoPacketType::ACK,
        ))
    }
    fn h0003_resethidio_nak(&mut self, _data: h0003::Nak) -> Result<(), CommandError> {
        Err(CommandError::IdNotImplemented(
            HidIoCommandID::ResetHidIo,
            HidIoPacketType::NAK,
        ))
    }
    fn h0003_resethidio_handler(&mut self, buf: HidIoPacketBuffer<H>) -> Result<(), CommandError> {
        // Handle packet type
        match buf.ptype {
            HidIoPacketType::Data => match self.h0003_resethidio_cmd(h0003::Cmd {}) {
                Ok(_ack) => self.empty_ack(buf.id),
                Err(_nak) => self.empty_nak(buf.id),
            },
            HidIoPacketType::NAData => Err(CommandError::InvalidPacketBufferType(buf.ptype)),
            HidIoPacketType::ACK => self.h0003_resethidio_ack(h0003::Ack {}),
            HidIoPacketType::NAK => self.h0003_resethidio_nak(h0003::Nak {}),
            _ => Ok(()),
        }
    }

    fn h0016_flashmode(&mut self, _data: h0016::Cmd) -> Result<(), CommandError> {
        self.tx_packetbuffer_send(&mut HidIoPacketBuffer {
            // Test packet id
            id: HidIoCommandID::FlashMode,
            // Detect max size
            max_len: self.default_packet_chunk(),
            // Ready
            done: true,
            // Use defaults for other fields
            ..Default::default()
        })
    }
    fn h0016_flashmode_cmd(&mut self, _data: h0016::Cmd) -> Result<h0016::Ack, h0016::Nak> {
        Err(h0016::Nak {
            error: h0016::Error::NotSupported,
        })
    }
    fn h0016_flashmode_ack(&mut self, _data: h0016::Ack) -> Result<(), CommandError> {
        Err(CommandError::IdNotImplemented(
            HidIoCommandID::FlashMode,
            HidIoPacketType::ACK,
        ))
    }
    fn h0016_flashmode_nak(&mut self, _data: h0016::Nak) -> Result<(), CommandError> {
        Err(CommandError::IdNotImplemented(
            HidIoCommandID::FlashMode,
            HidIoPacketType::NAK,
        ))
    }
    fn h0016_flashmode_handler(&mut self, buf: HidIoPacketBuffer<H>) -> Result<(), CommandError> {
        // Handle packet type
        match buf.ptype {
            HidIoPacketType::Data => match self.h0016_flashmode_cmd(h0016::Cmd {}) {
                Ok(ack) => self.short_ack(buf.id, ack.scancode),
                Err(nak) => self.byte_nak(buf.id, nak.error as u8),
            },
            HidIoPacketType::NAData => Err(CommandError::InvalidPacketBufferType(buf.ptype)),
            HidIoPacketType::ACK => {
                if buf.data.len() < 2 {
                    return Err(CommandError::DataVecNoData);
                }

                let scancode = u16::from_le_bytes(buf.data[0..2].try_into().unwrap());
                self.h0016_flashmode_ack(h0016::Ack { scancode })
            }
            HidIoPacketType::NAK => {
                if buf.data.len() < 1 {
                    return Err(CommandError::DataVecNoData);
                }

                let error = match h0016::Error::try_from(buf.data[0]) {
                    Ok(error) => error,
                    Err(_) => {
                        return Err(CommandError::InvalidProperty8(buf.data[0]));
                    }
                };
                self.h0016_flashmode_nak(h0016::Nak { error })
            }
            _ => Ok(()),
        }
    }

    fn h0017_unicodetext(&mut self, data: h0017::Cmd<H>, na: bool) -> Result<(), CommandError> {
        // Create appropriately sized buffer
        let mut buf = HidIoPacketBuffer {
            // Test packet id
            id: HidIoCommandID::UnicodeText,
            // Detect max size
            max_len: self.default_packet_chunk(),
            // Use defaults for other fields
            ..Default::default()
        };

        // Set NA (no-ack)
        if na {
            buf.ptype = HidIoPacketType::NAData;
        }

        // Build payload
        if !buf.append_payload(data.string.as_bytes()) {
            return Err(CommandError::DataVecTooSmall);
        }
        buf.done = true;

        self.tx_packetbuffer_send(&mut buf)
    }
    fn h0017_unicodetext_cmd(&mut self, _data: h0017::Cmd<H>) -> Result<h0017::Ack, h0017::Nak> {
        Err(h0017::Nak {})
    }
    fn h0017_unicodetext_nacmd(&mut self, _data: h0017::Cmd<H>) -> Result<(), CommandError> {
        Err(CommandError::IdNotImplemented(
            HidIoCommandID::UnicodeText,
            HidIoPacketType::NAData,
        ))
    }
    fn h0017_unicodetext_ack(&mut self, _data: h0017::Ack) -> Result<(), CommandError> {
        Err(CommandError::IdNotImplemented(
            HidIoCommandID::UnicodeText,
            HidIoPacketType::ACK,
        ))
    }
    fn h0017_unicodetext_nak(&mut self, _data: h0017::Nak) -> Result<(), CommandError> {
        Err(CommandError::IdNotImplemented(
            HidIoCommandID::UnicodeText,
            HidIoPacketType::NAK,
        ))
    }
    fn h0017_unicodetext_handler(&mut self, buf: HidIoPacketBuffer<H>) -> Result<(), CommandError> {
        // Handle packet type
        match buf.ptype {
            HidIoPacketType::Data => {
                // Copy data into struct
                let cmd = h0017::Cmd::<H> {
                    string: match String::from_utf8(buf.data) {
                        Ok(string) => string,
                        Err(e) => {
                            return Err(CommandError::InvalidUtf8(e));
                        }
                    },
                };

                match self.h0017_unicodetext_cmd(cmd) {
                    Ok(_ack) => self.empty_ack(buf.id),
                    Err(_nak) => self.empty_nak(buf.id),
                }
            }
            HidIoPacketType::NAData => {
                // Copy data into struct
                let cmd = h0017::Cmd::<H> {
                    string: match String::from_utf8(buf.data) {
                        Ok(string) => string,
                        Err(e) => {
                            return Err(CommandError::InvalidUtf8(e));
                        }
                    },
                };

                self.h0017_unicodetext_nacmd(cmd)
            }
            HidIoPacketType::ACK => self.h0017_unicodetext_ack(h0017::Ack {}),
            HidIoPacketType::NAK => self.h0017_unicodetext_nak(h0017::Nak {}),
            _ => Ok(()),
        }
    }

    fn h0018_unicodestate(&mut self, data: h0018::Cmd<H>, na: bool) -> Result<(), CommandError> {
        // Create appropriately sized buffer
        let mut buf = HidIoPacketBuffer {
            // Test packet id
            id: HidIoCommandID::UnicodeState,
            // Detect max size
            max_len: self.default_packet_chunk(),
            // Use defaults for other fields
            ..Default::default()
        };

        // Set NA (no-ack)
        if na {
            buf.ptype = HidIoPacketType::NAData;
        }

        // Build payload
        if !buf.append_payload(data.symbols.as_bytes()) {
            return Err(CommandError::DataVecTooSmall);
        }
        buf.done = true;

        self.tx_packetbuffer_send(&mut buf)
    }
    fn h0018_unicodestate_cmd(&mut self, _data: h0018::Cmd<H>) -> Result<h0018::Ack, h0018::Nak> {
        Err(h0018::Nak {})
    }
    fn h0018_unicodestate_nacmd(&mut self, _data: h0018::Cmd<H>) -> Result<(), CommandError> {
        Err(CommandError::IdNotImplemented(
            HidIoCommandID::UnicodeState,
            HidIoPacketType::NAData,
        ))
    }
    fn h0018_unicodestate_ack(&mut self, _data: h0018::Ack) -> Result<(), CommandError> {
        Err(CommandError::IdNotImplemented(
            HidIoCommandID::UnicodeState,
            HidIoPacketType::ACK,
        ))
    }
    fn h0018_unicodestate_nak(&mut self, _data: h0018::Nak) -> Result<(), CommandError> {
        Err(CommandError::IdNotImplemented(
            HidIoCommandID::UnicodeState,
            HidIoPacketType::NAK,
        ))
    }
    fn h0018_unicodestate_handler(
        &mut self,
        buf: HidIoPacketBuffer<H>,
    ) -> Result<(), CommandError> {
        // Handle packet type
        match buf.ptype {
            HidIoPacketType::Data => {
                // Copy data into struct
                let cmd = h0018::Cmd::<H> {
                    symbols: match String::from_utf8(buf.data) {
                        Ok(symbols) => symbols,
                        Err(e) => {
                            return Err(CommandError::InvalidUtf8(e));
                        }
                    },
                };

                match self.h0018_unicodestate_cmd(cmd) {
                    Ok(_ack) => self.empty_ack(buf.id),
                    Err(_nak) => self.empty_nak(buf.id),
                }
            }
            HidIoPacketType::NAData => {
                // Copy data into struct
                let cmd = h0018::Cmd::<H> {
                    symbols: match String::from_utf8(buf.data) {
                        Ok(symbols) => symbols,
                        Err(e) => {
                            return Err(CommandError::InvalidUtf8(e));
                        }
                    },
                };

                self.h0018_unicodestate_nacmd(cmd)
            }
            HidIoPacketType::ACK => self.h0018_unicodestate_ack(h0018::Ack {}),
            HidIoPacketType::NAK => self.h0018_unicodestate_nak(h0018::Nak {}),
            _ => Ok(()),
        }
    }

    fn h001a_sleepmode(&mut self, _data: h001a::Cmd) -> Result<(), CommandError> {
        self.tx_packetbuffer_send(&mut HidIoPacketBuffer {
            // Test packet id
            id: HidIoCommandID::SleepMode,
            // Detect max size
            max_len: self.default_packet_chunk(),
            // Ready
            done: true,
            // Use defaults for other fields
            ..Default::default()
        })
    }
    fn h001a_sleepmode_cmd(&mut self, _data: h001a::Cmd) -> Result<h001a::Ack, h001a::Nak> {
        Err(h001a::Nak {
            error: h001a::Error::NotSupported,
        })
    }
    fn h001a_sleepmode_ack(&mut self, _data: h001a::Ack) -> Result<(), CommandError> {
        Err(CommandError::IdNotImplemented(
            HidIoCommandID::FlashMode,
            HidIoPacketType::ACK,
        ))
    }
    fn h001a_sleepmode_nak(&mut self, _data: h001a::Nak) -> Result<(), CommandError> {
        Err(CommandError::IdNotImplemented(
            HidIoCommandID::FlashMode,
            HidIoPacketType::NAK,
        ))
    }
    fn h001a_sleepmode_handler(&mut self, buf: HidIoPacketBuffer<H>) -> Result<(), CommandError> {
        // Handle packet type
        match buf.ptype {
            HidIoPacketType::Data => match self.h001a_sleepmode_cmd(h001a::Cmd {}) {
                Ok(_ack) => self.empty_ack(buf.id),
                Err(nak) => self.byte_nak(buf.id, nak.error as u8),
            },
            HidIoPacketType::NAData => Err(CommandError::InvalidPacketBufferType(buf.ptype)),
            HidIoPacketType::ACK => self.h001a_sleepmode_ack(h001a::Ack {}),
            HidIoPacketType::NAK => {
                if buf.data.len() < 1 {
                    return Err(CommandError::DataVecNoData);
                }

                let error = match h001a::Error::try_from(buf.data[0]) {
                    Ok(error) => error,
                    Err(_) => {
                        return Err(CommandError::InvalidProperty8(buf.data[0]));
                    }
                };
                self.h001a_sleepmode_nak(h001a::Nak { error })
            }
            _ => Ok(()),
        }
    }

    fn h0031_terminalcmd(&mut self, data: h0031::Cmd<H>, na: bool) -> Result<(), CommandError> {
        // Create appropriately sized buffer
        let mut buf = HidIoPacketBuffer {
            // Test packet id
            id: HidIoCommandID::TerminalCmd,
            // Detect max size
            max_len: self.default_packet_chunk(),
            // Use defaults for other fields
            ..Default::default()
        };

        // Set NA (no-ack)
        if na {
            buf.ptype = HidIoPacketType::NAData;
        }

        // Build payload
        if !buf.append_payload(&data.command.as_bytes()) {
            return Err(CommandError::DataVecTooSmall);
        }
        buf.done = true;

        self.tx_packetbuffer_send(&mut buf)
    }
    fn h0031_terminalcmd_cmd(&mut self, _data: h0031::Cmd<H>) -> Result<h0031::Ack, h0031::Nak> {
        Err(h0031::Nak {})
    }
    fn h0031_terminalcmd_nacmd(&mut self, _data: h0031::Cmd<H>) -> Result<(), CommandError> {
        Err(CommandError::IdNotImplemented(
            HidIoCommandID::TerminalCmd,
            HidIoPacketType::NAData,
        ))
    }
    fn h0031_terminalcmd_ack(&mut self, _data: h0031::Ack) -> Result<(), CommandError> {
        Err(CommandError::IdNotImplemented(
            HidIoCommandID::TerminalCmd,
            HidIoPacketType::ACK,
        ))
    }
    fn h0031_terminalcmd_nak(&mut self, _data: h0031::Nak) -> Result<(), CommandError> {
        Err(CommandError::IdNotImplemented(
            HidIoCommandID::TerminalCmd,
            HidIoPacketType::NAK,
        ))
    }
    fn h0031_terminalcmd_handler(&mut self, buf: HidIoPacketBuffer<H>) -> Result<(), CommandError> {
        // Handle packet type
        match buf.ptype {
            HidIoPacketType::Data => {
                // Copy data into struct
                let cmd = h0031::Cmd::<H> {
                    command: match String::from_utf8(buf.data) {
                        Ok(string) => string,
                        Err(e) => {
                            return Err(CommandError::InvalidUtf8(e));
                        }
                    },
                };

                match self.h0031_terminalcmd_cmd(cmd) {
                    Ok(_ack) => self.empty_ack(buf.id),
                    Err(_nak) => self.empty_nak(buf.id),
                }
            }
            HidIoPacketType::NAData => {
                // Copy data into struct
                let cmd = h0031::Cmd::<H> {
                    command: match String::from_utf8(buf.data) {
                        Ok(string) => string,
                        Err(e) => {
                            return Err(CommandError::InvalidUtf8(e));
                        }
                    },
                };

                self.h0031_terminalcmd_nacmd(cmd)
            }
            HidIoPacketType::ACK => self.h0031_terminalcmd_ack(h0031::Ack {}),
            HidIoPacketType::NAK => self.h0031_terminalcmd_nak(h0031::Nak {}),
            _ => Ok(()),
        }
    }

    fn h0034_terminalout(&mut self, data: h0034::Cmd<H>, na: bool) -> Result<(), CommandError> {
        // Create appropriately sized buffer
        let mut buf = HidIoPacketBuffer {
            // Test packet id
            id: HidIoCommandID::TerminalOut,
            // Detect max size
            max_len: self.default_packet_chunk(),
            // Use defaults for other fields
            ..Default::default()
        };

        // Set NA (no-ack)
        if na {
            buf.ptype = HidIoPacketType::NAData;
        }

        // Build payload
        if !buf.append_payload(&data.output.as_bytes()) {
            return Err(CommandError::DataVecTooSmall);
        }
        buf.done = true;

        self.tx_packetbuffer_send(&mut buf)
    }
    fn h0034_terminalout_cmd(&mut self, _data: h0034::Cmd<H>) -> Result<h0034::Ack, h0034::Nak> {
        Err(h0034::Nak {})
    }
    fn h0034_terminalout_nacmd(&mut self, _data: h0034::Cmd<H>) -> Result<(), CommandError> {
        Err(CommandError::IdNotImplemented(
            HidIoCommandID::TerminalOut,
            HidIoPacketType::NAData,
        ))
    }
    fn h0034_terminalout_ack(&mut self, _data: h0034::Ack) -> Result<(), CommandError> {
        Err(CommandError::IdNotImplemented(
            HidIoCommandID::TerminalOut,
            HidIoPacketType::ACK,
        ))
    }
    fn h0034_terminalout_nak(&mut self, _data: h0034::Nak) -> Result<(), CommandError> {
        Err(CommandError::IdNotImplemented(
            HidIoCommandID::TerminalOut,
            HidIoPacketType::NAK,
        ))
    }
    fn h0034_terminalout_handler(&mut self, buf: HidIoPacketBuffer<H>) -> Result<(), CommandError> {
        // Handle packet type
        match buf.ptype {
            HidIoPacketType::Data => {
                // Copy data into struct
                let cmd = h0034::Cmd::<H> {
                    output: match String::from_utf8(buf.data) {
                        Ok(string) => string,
                        Err(e) => {
                            return Err(CommandError::InvalidUtf8(e));
                        }
                    },
                };

                match self.h0034_terminalout_cmd(cmd) {
                    Ok(_ack) => self.empty_ack(buf.id),
                    Err(_nak) => self.empty_nak(buf.id),
                }
            }
            HidIoPacketType::NAData => {
                // Copy data into struct
                let cmd = h0034::Cmd::<H> {
                    output: match String::from_utf8(buf.data) {
                        Ok(string) => string,
                        Err(e) => {
                            return Err(CommandError::InvalidUtf8(e));
                        }
                    },
                };

                self.h0034_terminalout_nacmd(cmd)
            }
            HidIoPacketType::ACK => self.h0034_terminalout_ack(h0034::Ack {}),
            HidIoPacketType::NAK => self.h0034_terminalout_nak(h0034::Nak {}),
            _ => Ok(()),
        }
    }

    fn h0050_manufacturing(&mut self, data: h0050::Cmd) -> Result<(), CommandError> {
        // Create appropriately sized buffer
        let mut buf = HidIoPacketBuffer {
            // Test packet id
            id: HidIoCommandID::ManufacturingTest,
            // Detect max size
            max_len: self.default_packet_chunk(),
            // Use defaults for other fields
            ..Default::default()
        };

        // Build payload
        if !buf.append_payload(&data.command.to_le_bytes()) {
            return Err(CommandError::DataVecTooSmall);
        }
        if !buf.append_payload(&data.argument.to_le_bytes()) {
            return Err(CommandError::DataVecTooSmall);
        }

        buf.done = true;

        self.tx_packetbuffer_send(&mut buf)
    }
    fn h0050_manufacturing_cmd(
        &mut self,
        _data: h0050::Cmd,
    ) -> Result<h0050::Ack<H>, h0050::Nak<H>> {
        Err(h0050::Nak { data: Vec::new() })
    }
    fn h0050_manufacturing_ack(&mut self, _data: h0050::Ack<H>) -> Result<(), CommandError> {
        Err(CommandError::IdNotImplemented(
            HidIoCommandID::ManufacturingTest,
            HidIoPacketType::ACK,
        ))
    }
    fn h0050_manufacturing_nak(&mut self, _data: h0050::Nak<H>) -> Result<(), CommandError> {
        Err(CommandError::IdNotImplemented(
            HidIoCommandID::ManufacturingTest,
            HidIoPacketType::NAK,
        ))
    }
    fn h0050_manufacturing_handler(
        &mut self,
        buf: HidIoPacketBuffer<H>,
    ) -> Result<(), CommandError> {
        // Handle packet type
        match buf.ptype {
            HidIoPacketType::Data => {
                if buf.data.len() < 4 {
                    return Err(CommandError::DataVecNoData);
                }

                // Retrieve fields
                let command = u16::from_le_bytes(buf.data[0..2].try_into().unwrap());
                let argument = u16::from_le_bytes(buf.data[2..4].try_into().unwrap());

                match self.h0050_manufacturing_cmd(h0050::Cmd { command, argument }) {
                    Ok(ack) => {
                        // Build ACK (max test data size)
                        let mut buf = HidIoPacketBuffer {
                            // Data packet
                            ptype: HidIoPacketType::ACK,
                            // Packet id
                            id: buf.id,
                            // Detect max size
                            max_len: self.default_packet_chunk(),
                            ..Default::default()
                        };

                        // Copy data into buffer
                        if !buf.append_payload(&ack.data) {
                            return Err(CommandError::DataVecTooSmall);
                        }
                        buf.done = true;
                        self.tx_packetbuffer_send(&mut buf)
                    }
                    Err(nak) => {
                        // Build ACK (max test data size)
                        let mut buf = HidIoPacketBuffer {
                            // Data packet
                            ptype: HidIoPacketType::NAK,
                            // Packet id
                            id: buf.id,
                            // Detect max size
                            max_len: self.default_packet_chunk(),
                            ..Default::default()
                        };

                        // Copy data into buffer
                        if !buf.append_payload(&nak.data) {
                            return Err(CommandError::DataVecTooSmall);
                        }
                        buf.done = true;
                        self.tx_packetbuffer_send(&mut buf)
                    }
                }
            }
            HidIoPacketType::NAData => Err(CommandError::InvalidPacketBufferType(buf.ptype)),
            HidIoPacketType::ACK => {
                // Copy data into struct
                let ack = h0050::Ack::<H> {
                    data: match Vec::from_slice(&buf.data) {
                        Ok(data) => data,
                        Err(_) => {
                            return Err(CommandError::DataVecTooSmall);
                        }
                    },
                };

                self.h0050_manufacturing_ack(ack)
            }
            HidIoPacketType::NAK => {
                // Copy data into struct
                let nak = h0050::Nak::<H> {
                    data: match Vec::from_slice(&buf.data) {
                        Ok(data) => data,
                        Err(_) => {
                            return Err(CommandError::DataVecTooSmall);
                        }
                    },
                };

                self.h0050_manufacturing_nak(nak)
            }
            _ => Ok(()),
        }
    }
}
