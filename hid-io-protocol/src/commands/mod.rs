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
use core::convert::TryInto;
use core::ops::{Add, Mul};
use heapless::consts::{U0, U1, U256, U257, U261, U262, U4, U5};
use heapless::{String, Vec};
use typenum::{Prod, Sum, Unsigned};

#[cfg(feature = "server")]
use log::{debug, error};

// ----- Modules -----

mod test;

// ----- Macros -----

// ----- Enumerations -----

#[derive(Debug)]
pub enum CommandError {
    DataVecNoData,
    DataVecTooSmall,
    IdNotImplemented(HidIoCommandID),
    IdNotMatched(HidIoCommandID),
    IdNotSupported(HidIoCommandID),
    IdVecTooSmall,
    InvalidId(u32),
    InvalidProperty8(u8),
    InvalidRxMessage(HidIoPacketType),
    InvalidUtf8(core::str::Utf8Error),
    PacketDecodeError(HidIoParseError),
    SerializationFailed(HidIoParseError),
    TestFailure,
    TxBufferSendFailed,
    TxBufferVecTooSmall,
}

// ----- Command Structs -----

/// Supported Ids
pub mod h0000 {
    use super::super::HidIoCommandID;
    use heapless::{ArrayLength, Vec};

    pub struct Cmd {}

    pub struct Ack<ID: ArrayLength<HidIoCommandID>> {
        pub ids: Vec<HidIoCommandID, ID>,
    }

    pub struct Nak {}
}

/// Info Query
pub mod h0001 {
    use heapless::consts::U256;
    use heapless::String;
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

    pub struct Cmd {
        pub property: Property,
    }

    pub struct Ack {
        pub property: Property,

        /// OS Type field
        pub os: OSType,

        /// Number is set when the given property specifies a number
        pub number: u16,

        /// String is set when the given property specifies a string
        pub string: String<U256>,
    }

    pub struct Nak {
        pub property: Property,
    }
}

/// Test Message
pub mod h0002 {
    use heapless::{ArrayLength, Vec};

    pub struct Cmd<D: ArrayLength<u8>> {
        pub data: Vec<u8, D>,
    }

    pub struct Ack<D: ArrayLength<u8>> {
        pub data: Vec<u8, D>,
    }

    pub struct Nak {}
}

/// Reset HID-IO
/// TODO
pub mod h0003 {
    pub struct Cmd {}
    pub struct Ack {}
    pub struct Nak {}
}

/// Get Properties
/// TODO
pub mod h0010 {
    pub struct Cmd {}
    pub struct Ack {}
    pub struct Nak {}
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
/// TODO
pub mod h0016 {
    pub struct Cmd {}
    pub struct Ack {}
    pub struct Nak {}
}

/// UTF-8 Character Stream
/// TODO
pub mod h0017 {
    pub struct Cmd {}
    pub struct Ack {}
    pub struct Nak {}
}

/// UTF-8 State
/// TODO
pub mod h0018 {
    pub struct Cmd {}
    pub struct Ack {}
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
/// TODO
pub mod h001a {
    pub struct Cmd {}
    pub struct Ack {}
    pub struct Nak {}
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
/// TODO
pub mod h0031 {
    pub struct Cmd {}
    pub struct Ack {}
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
/// TODO
pub mod h0034 {
    pub struct Cmd {}
    pub struct Ack {}
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
/// TODO
pub mod h0050 {
    pub struct Cmd {}
    pub struct Ack {}
    pub struct Nak {}
}

// ----- Traits -----

/// HID-IO Command Interface
///
/// The HID-IO command interface requires 4 different buffers
/// (which can be broken into 2 categories)
/// - Byte buffers
///   * Tx Byte Buffer
///   * Rx Byte Buffer
/// - Data buffer
///   * Rx Data Buffer
///   * Rx Ack Buffer
///
/// Byte buffers handle the incoming byte streams.
/// The byte streams must be the same as as the incoming interface.
/// Common sizes include:
/// - 7 bytes (USB 2.0 LS /w HID ID byte)
/// - 8 bytes (USB 2.0 LS)
/// - 63 bytes (USB 2.0 FS /w HID ID byte)
/// - 64 bytes (USB 2.0 FS)
/// - 1023 bytes (USB 2.0 HS /w HID ID byte)
/// - 1024 bytes (USB 2.0 HS)
///
/// The data buffers are use to reconstruct continued HID-IO messages
/// into a usable message buffer.
/// As continued data packets can have significant delays we have to
/// wait for all the data to arrive before handling the message.
/// The data buffer should be sized to the largest possible continued
/// packet possible.
/// If the Rx data buffer is not large enough a NAK will be returned
/// as well as an error.
/// If the Rx ack buffer is not large enough, an error will be raised.
///
/// These buffer limits are necessary to handle memory constraints of
/// embedded MCUs.
/// Try to avoid making server implementations too constrained as
/// devices will likely have a wide range of buffer limits.
trait Commands<
    TX: ArrayLength<Vec<u8, N>>,
    RX: ArrayLength<Vec<u8, N>>,
    N: ArrayLength<u8>,
    H: ArrayLength<u8>,
    ID: ArrayLength<HidIoCommandID> + ArrayLength<u8> + Mul<U4> + Add<U5>,
> where
    <ID as Mul<U4>>::Output: Add<U5>,
    H: core::fmt::Debug,
{
    /// Special generic to handle the supported id serialization
    /// buffer
    /// ID * 4 + 5
    /// CommandIds are sent as 32-bit unsigned and 5 additional bytes
    /// are needed for the header
    type ID32 = Sum<Prod<ID, U4>, U5>;

    fn tx_bytebuffer(&mut self) -> &mut buffer::Buffer<TX, N>;
    fn rx_bytebuffer(&mut self) -> &mut buffer::Buffer<RX, N>;
    fn rx_packetbuffer(&self) -> &HidIoPacketBuffer<H>;
    fn rx_packetbuffer_mut(&mut self) -> &mut HidIoPacketBuffer<H>;
    fn rx_packetbuffer_clear(&mut self);
    fn supported_id(&self, id: HidIoCommandID) -> bool;

    /// Process incoming rx byte buffer
    /// buffer_limit defines the maximum number of HidIoPacketBuffer s
    /// to process before returning Ok.
    /// This is useful on resource constrained single threaded MCUs.
    /// Set buffer_limit to 0 to process until rx byte buffer is empty
    ///
    /// The number of processed buffers is returned if successful.
    /// If non-zero, indicates that the link is not idle
    fn process_rx(&mut self, buffer_limit: u8) -> Result<u8, CommandError>
    where
        <Self as Commands<TX, RX, N, H, ID>>::ID32: ArrayLength<u8>,
    {
        let mut buffer_count = 0;
        while buffer_limit == 0 || buffer_count < buffer_limit {
            // Retrieve vec chunk
            if let Some(buf) = self.rx_bytebuffer().dequeue() {
                // Decode chunk
                match self.rx_packetbuffer_mut().decode_packet(&buf) {
                    Ok(_recv) => {
                        buffer_count += 1;

                        // Handle packet type
                        match self.rx_packetbuffer().ptype {
                            HidIoPacketType::Sync => {
                                debug!("Sync. Resetting buffer");
                                self.rx_packetbuffer_clear();
                            }
                            HidIoPacketType::ACK => {}
                            HidIoPacketType::NAK => {}
                            HidIoPacketType::Continued | HidIoPacketType::Data => {}
                            HidIoPacketType::NAData | HidIoPacketType::NAContinued => {}
                        }
                    }
                    Err(e) => {
                        error!("Decode error: {:?} {:?}", e, buf);
                        return Err(CommandError::PacketDecodeError(e));
                    }
                }

                // Handle buffer if ready
                if self.rx_packetbuffer().done {
                    self.rx_message_handling()?;
                    self.rx_packetbuffer_clear();
                }
            } else {
                break;
            }
        }

        Ok(buffer_count)
    }

    /// Simple empty ack
    fn empty_ack(&mut self) -> Result<(), CommandError> {
        // Build empty ACK
        let mut buf = HidIoPacketBuffer::<U0> {
            // Data packet
            ptype: HidIoPacketType::ACK,
            // Packet id
            id: self.rx_packetbuffer().id,
            // Detect max size
            max_len: <N as Unsigned>::to_u32(),
            // Ready to go
            done: true,
            // Use defaults for other fields
            ..Default::default()
        };

        // Serialize buffer
        let mut data = [0u8; 5];
        self.send_buffer(&mut data, &mut buf)
    }

    /// Simple empty nak
    fn empty_nak(&mut self) -> Result<(), CommandError> {
        // Build empty NAK
        let mut buf = HidIoPacketBuffer::<U0> {
            // Data packet
            ptype: HidIoPacketType::NAK,
            // Packet id
            id: self.rx_packetbuffer().id,
            // Detect max size
            max_len: <N as Unsigned>::to_u32(),
            // Ready to go
            done: true,
            // Use defaults for other fields
            ..Default::default()
        };

        // Serialize buffer
        let mut data = [0u8; 5];
        self.send_buffer(&mut data, &mut buf)
    }

    /// Simple byte ack
    fn byte_ack(&mut self, byte: u8) -> Result<(), CommandError> {
        // Build ACK
        let mut buf = HidIoPacketBuffer::<U1> {
            // Data packet
            ptype: HidIoPacketType::ACK,
            // Packet id
            id: self.rx_packetbuffer().id,
            // Detect max size
            max_len: <N as Unsigned>::to_u32(),
            // Byte payload
            data: Vec::from_slice(&[byte]).unwrap(),
            // Ready to go
            done: true,
        };

        // Serialize buffer
        let mut data = [0u8; 6];
        self.send_buffer(&mut data, &mut buf)
    }

    /// Simple byte nak
    fn byte_nak(&mut self, byte: u8) -> Result<(), CommandError> {
        // Build NAK
        let mut buf = HidIoPacketBuffer::<U1> {
            // Data packet
            ptype: HidIoPacketType::NAK,
            // Packet id
            id: self.rx_packetbuffer().id,
            // Detect max size
            max_len: <N as Unsigned>::to_u32(),
            // Byte payload
            data: Vec::from_slice(&[byte]).unwrap(),
            // Ready to go
            done: true,
        };

        // Serialize buffer
        let mut data = [0u8; 6];
        self.send_buffer(&mut data, &mut buf)
    }

    /// Serialize and send buffer
    ///
    /// data must be an array that can fit the serialized output
    /// from buf
    fn send_buffer<L: ArrayLength<u8>>(
        &mut self,
        data: &mut [u8],
        buf: &mut HidIoPacketBuffer<L>,
    ) -> Result<(), CommandError> {
        // Serialize
        let data = match buf.serialize_buffer(data) {
            Ok(data) => data,
            Err(err) => {
                return Err(CommandError::SerializationFailed(err));
            }
        };

        // Add serialized data to buffer
        // May need to enqueue multiple packets depending how much
        // was serialized
        for pos in (0..data.len()).step_by(<N as Unsigned>::to_usize()) {
            let len = core::cmp::min(<N as Unsigned>::to_usize(), data.len() - pos);
            match self
                .tx_bytebuffer()
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

    /// Process specific packet types
    /// Handles matching to interface functions
    fn rx_message_handling(&mut self) -> Result<(), CommandError>
    where
        <Self as Commands<TX, RX, N, H, ID>>::ID32: ArrayLength<u8>,
    {
        let buf = self.rx_packetbuffer();

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
            HidIoCommandID::SupportedIDs => self.h0000_supported_ids_handler(),
            HidIoCommandID::GetInfo => self.h0001_info_handler(),
            HidIoCommandID::TestPacket => self.h0002_test_handler(),
            _ => Err(CommandError::IdNotMatched(buf.id)),
        }
    }

    fn h0000_supported_ids(&mut self, _data: h0000::Cmd) -> Result<(), CommandError> {
        // Create appropriately sized buffer
        let mut buf = HidIoPacketBuffer::<U0> {
            // Test packet id
            id: HidIoCommandID::SupportedIDs,
            // Detect max size
            max_len: <N as Unsigned>::to_u32(),
            // Ready to go
            done: true,
            // Use defaults for other fields
            ..Default::default()
        };

        // Serialize buffer
        let mut data = [0u8; 5];
        self.send_buffer(&mut data, &mut buf)
    }
    fn h0000_supported_ids_cmd(&self, _data: h0000::Cmd) -> Result<h0000::Ack<ID>, h0000::Nak> {
        Err(h0000::Nak {})
    }
    fn h0000_supported_ids_ack(&self, _data: h0000::Ack<ID>) -> Result<(), CommandError> {
        Err(CommandError::IdNotImplemented(HidIoCommandID::SupportedIDs))
    }
    fn h0000_supported_ids_nak(&self, _data: h0000::Nak) -> Result<(), CommandError> {
        Err(CommandError::IdNotImplemented(HidIoCommandID::SupportedIDs))
    }
    fn h0000_supported_ids_handler(&mut self) -> Result<(), CommandError>
    where
        <Self as Commands<TX, RX, N, H, ID>>::ID32: ArrayLength<u8>,
    {
        let buf = self.rx_packetbuffer();

        // Handle packet type
        match buf.ptype {
            HidIoPacketType::Data | HidIoPacketType::NAData => {
                match self.h0000_supported_ids_cmd(h0000::Cmd {}) {
                    Ok(ack) => {
                        // Build ACK
                        let mut buf = HidIoPacketBuffer::<Self::ID32> {
                            // Data packet
                            ptype: HidIoPacketType::ACK,
                            // Packet id
                            id: self.rx_packetbuffer().id,
                            // Detect max size
                            max_len: <N as Unsigned>::to_u32(),
                            // Ready to go
                            done: true,
                            // Use defaults for other fields
                            ..Default::default()
                        };

                        // Build list of ids
                        for id in ack.ids {
                            if buf
                                .data
                                .extend_from_slice(&(id as u32).to_le_bytes())
                                .is_err()
                            {
                                return Err(CommandError::IdVecTooSmall);
                            }
                        }

                        // Allocate serialization buffer
                        // This is a bit complicated as the list
                        // of IDs is variable at compile time
                        // See type ID32 for the specific size
                        let mut data: Vec<u8, Self::ID32> = Vec::new();
                        data.resize_default(<Self::ID32 as Unsigned>::to_usize())
                            .unwrap();

                        // Serialize buffer
                        self.send_buffer(&mut data, &mut buf)
                    }
                    Err(_nak) => self.empty_nak(),
                }
            }
            HidIoPacketType::ACK => {
                // Retrieve list of ids
                let mut ids: Vec<HidIoCommandID, ID> = Vec::new();
                // Ids are always 32-bit le
                let mut pos = 0;
                while pos <= buf.data.len() - 4 {
                    let slice = &buf.data[pos..pos + 4];
                    let idnum = u32::from_le_bytes(slice.try_into().unwrap());
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
                    pos += 4;
                }
                self.h0000_supported_ids_ack(h0000::Ack { ids })
            }
            HidIoPacketType::NAK => self.h0000_supported_ids_nak(h0000::Nak {}),
            _ => Ok(()),
        }
    }

    fn h0001_info(&mut self, data: h0001::Cmd) -> Result<(), CommandError> {
        // Create appropriately sized buffer
        let mut buf = HidIoPacketBuffer::<U1> {
            // Test packet id
            id: HidIoCommandID::GetInfo,
            // Detect max size
            max_len: <N as Unsigned>::to_u32(),
            // Ready to go
            done: true,
            // Use defaults for other fields
            ..Default::default()
        };

        // Encode property
        if buf.data.push(data.property as u8).is_err() {
            return Err(CommandError::DataVecTooSmall);
        }

        // Serialize buffer
        let mut data = [0u8; 6];
        self.send_buffer(&mut data, &mut buf)
    }
    fn h0001_info_cmd(&self, _data: h0001::Cmd) -> Result<h0001::Ack, h0001::Nak> {
        Err(h0001::Nak {
            property: h0001::Property::Unknown,
        })
    }
    fn h0001_info_ack(&self, _data: h0001::Ack) -> Result<(), CommandError> {
        Err(CommandError::IdNotImplemented(HidIoCommandID::SupportedIDs))
    }
    fn h0001_info_nak(&self, _data: h0001::Nak) -> Result<(), CommandError> {
        Err(CommandError::IdNotImplemented(HidIoCommandID::SupportedIDs))
    }
    fn h0001_info_handler(&mut self) -> Result<(), CommandError> {
        let buf = self.rx_packetbuffer();

        // Handle packet type
        match buf.ptype {
            HidIoPacketType::Data | HidIoPacketType::NAData => {
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
                        // Build ACK (max string + 1)
                        let mut buf = HidIoPacketBuffer::<U257> {
                            // Data packet
                            ptype: HidIoPacketType::ACK,
                            // Packet id
                            id: self.rx_packetbuffer().id,
                            // Detect max size
                            max_len: <N as Unsigned>::to_u32(),
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

                        // Allocate serialization buffer
                        // 257 + 5 = 262
                        // Only send necessary size though
                        let mut data: Vec<u8, U262> = Vec::new();
                        data.resize_default(buf.serialized_len() as usize).unwrap();

                        // Serialize buffer
                        self.send_buffer(&mut data, &mut buf)
                    }
                    Err(_nak) => self.byte_nak(property as u8),
                }
            }
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
                        let bytes: Vec<u8, U256> = Vec::from_slice(&buf.data[1..]).unwrap();
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

    fn h0002_test(&mut self, data: h0002::Cmd<U256>) -> Result<(), CommandError> {
        // Create appropriately sized buffer
        let mut buf = HidIoPacketBuffer::<U256> {
            // Test packet id
            id: HidIoCommandID::TestPacket,
            // Detect max size
            max_len: <N as Unsigned>::to_u32(),
            // Use defaults for other fields
            ..Default::default()
        };

        // Build payload
        if !buf.append_payload(&data.data) {
            return Err(CommandError::DataVecTooSmall);
        }
        buf.done = true;

        // 256 + 5 = 261
        // Only send necessary size though
        let mut data: Vec<u8, U261> = Vec::new();
        data.resize_default(buf.serialized_len() as usize).unwrap();
        self.send_buffer(&mut data, &mut buf)
    }
    fn h0002_test_cmd(&self, _data: h0002::Cmd<U256>) -> Result<h0002::Ack<U256>, h0002::Nak> {
        Err(h0002::Nak {})
    }
    fn h0002_test_ack(&self, _data: h0002::Ack<U256>) -> Result<(), CommandError> {
        Err(CommandError::IdNotImplemented(HidIoCommandID::SupportedIDs))
    }
    fn h0002_test_nak(&self, _data: h0002::Nak) -> Result<(), CommandError> {
        Err(CommandError::IdNotImplemented(HidIoCommandID::SupportedIDs))
    }
    fn h0002_test_handler(&mut self) -> Result<(), CommandError> {
        let buf = self.rx_packetbuffer();

        // Handle packet type
        match buf.ptype {
            HidIoPacketType::Data | HidIoPacketType::NAData => {
                // Copy data into struct
                let cmd = h0002::Cmd::<U256> {
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
                        let mut buf = HidIoPacketBuffer::<U256> {
                            // Data packet
                            ptype: HidIoPacketType::ACK,
                            // Packet id
                            id: self.rx_packetbuffer().id,
                            // Detect max size
                            max_len: <N as Unsigned>::to_u32(),
                            ..Default::default()
                        };

                        // Copy data into buffer
                        if !buf.append_payload(&ack.data) {
                            return Err(CommandError::DataVecTooSmall);
                        }
                        buf.done = true;

                        // Allocate serialization buffer
                        // 256 + 5 = 261
                        // Only send necessary size though
                        let mut data: Vec<u8, U261> = Vec::new();
                        data.resize_default(buf.serialized_len() as usize).unwrap();

                        // Serialize buffer
                        self.send_buffer(&mut data, &mut buf)
                    }
                    Err(_nak) => self.empty_nak(),
                }
            }
            HidIoPacketType::ACK => {
                // Copy data into struct
                let ack = h0002::Ack::<U256> {
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
        // Create appropriately sized buffer
        let mut buf = HidIoPacketBuffer::<U0> {
            // Test packet id
            id: HidIoCommandID::ResetHidIo,
            // Detect max size
            max_len: <N as Unsigned>::to_u32(),
            // Ready
            done: true,
            // Use defaults for other fields
            ..Default::default()
        };

        // Serialize buffer
        let mut data = [0u8; 5];
        self.send_buffer(&mut data, &mut buf)
    }
    fn h0003_resethidio_cmd(&self, _data: h0003::Cmd) -> Result<h0003::Ack, h0003::Nak> {
        Err(h0003::Nak {})
    }
    fn h0003_resethidio_ack(&self, _data: h0003::Ack) -> Result<(), CommandError> {
        Err(CommandError::IdNotImplemented(HidIoCommandID::SupportedIDs))
    }
    fn h0003_resethidio_nak(&self, _data: h0003::Nak) -> Result<(), CommandError> {
        Err(CommandError::IdNotImplemented(HidIoCommandID::SupportedIDs))
    }
    fn h0003_resethidio_handler(&mut self) -> Result<(), CommandError> {
        let buf = self.rx_packetbuffer();

        // Handle packet type
        match buf.ptype {
            HidIoPacketType::Data | HidIoPacketType::NAData => {
                match self.h0003_resethidio_cmd(h0003::Cmd {}) {
                    Ok(_ack) => self.empty_ack(),
                    Err(_nak) => self.empty_nak(),
                }
            }
            HidIoPacketType::ACK => self.h0003_resethidio_ack(h0003::Ack {}),
            HidIoPacketType::NAK => self.h0003_resethidio_nak(h0003::Nak {}),
            _ => Ok(()),
        }
    }
}
