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
use heapless::consts::{U0, U1, U256, U257, U262, U4, U5};
use heapless::{String, Vec};
use typenum::{Prod, Sum, Unsigned};

#[cfg(feature = "server")]
use log::{debug, error, warn};

// ----- Modules -----

// ----- Macros -----

// ----- Enumerations -----

#[derive(Debug)]
pub enum CommandError {
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
    TxBufferSendFailed,
}

// ----- Command Structs -----

pub mod h0000 {
    use super::super::HidIoCommandID;
    use heapless::{ArrayLength, Vec};

    pub struct Cmd {}

    pub struct Ack<ID: ArrayLength<HidIoCommandID>> {
        pub ids: Vec<HidIoCommandID, ID>,
    }

    pub struct Nak {}
}

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

/*
struct H0002TestCmd {}

struct H0002TestAck {}

struct H0003ResetHIDIOCmd {}

struct H0003ResetHIDIOAck {}

struct H0010GetPropertiesCmd {}

struct H0010GetPropertiesAck {}

struct H0011USBKeyStateCmd {}

struct H0011USBKeyStateAck {}

struct H0012KeyboardLayoutCmd {}

struct H0012KeyboardLayoutAck {}

struct H0013ButtonLayoutCmd {}

struct H0013ButtonLayoutAck {}

struct H0014KeycapTypesCmd {}

struct H0014KeycapTypesAck {}

struct H0015LEDLayoutCmd {}

struct H0015LEDLayoutAck {}

struct H0016FlashModeCmd {}

struct H0016FlashModeAck {}

struct H0017UTF8CharacterStreamCmd {}

struct H0017UTF8CharacterStreamAck {}

struct H0018UTF8StateCmd {}

struct H0018UTF8StateAck {}

struct H0019TriggerHostMacroCmd {}

struct H0019TriggerHostMacroAck {}

struct H001ASleepModeCmd {}

struct H001ASleepModeAck {}

struct H0020KLLTriggerStateCmd {}

struct H0020KLLTriggerStateAck {}

struct H0021PixelSettingCmd {}

struct H0021PixelSettingAck {}

struct HOO22PixelSet1c8bCmd {}

struct HOO22PixelSet1c8bAck {}

struct H0023PixelSet3c8bCmd {}

struct H0023PixelSet3c8bAck {}

struct H0024PixelSet1c16bCmd {}

struct H0024PixelSet1c16bAck {}

struct H0025PixelSet3c16bCmd {}

struct H0025PixelSet3c16bAck {}

struct H0030OpenURLCmd {}

struct H0030OpenURLAck {}

struct H0031TerminalCmdCmd {}

struct H0031TerminalCmdAck {}

struct H0032GetOSLayoutCmd {}

struct H0032GetOSLayoutAck {}

struct H0033SetOSLayoutCmd {}

struct H0033SetOSLayoutAck {}

struct H0034TerminalOutCmd {}

struct H0034TerminalOutAck {}

struct H0040HIDKeyboardStateCmd {}

struct H0040HIDKeyboardStateAck {}

struct H0041HIDKeyboardLEDStateCmd {}

struct H0041HIDKeyboardLEDStateAck {}

struct H0042HIDMouseStateCmd {}

struct H0042HIDMouseStateAck {}

struct H0043HIDJoystickStateCmd {}

struct H0043HIDJoystickStateAck {}

struct H0050ManufacturingTestCmd {}

struct H0050ManufacturingTestAck {}
*/

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
                            HidIoPacketType::ACK => {
                                // Don't ack an ack
                            }
                            HidIoPacketType::NAK => {
                                warn!("NACK. Resetting buffer");
                                self.rx_packetbuffer_clear();
                            }
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
        match self.tx_bytebuffer().enqueue(Vec::from_slice(data).unwrap()) {
            Ok(_) => Ok(()),
            Err(_) => Err(CommandError::TxBufferSendFailed),
        }
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
    fn h0001_info_handler(&mut self) -> Result<(), CommandError>
    where
        <Self as Commands<TX, RX, N, H, ID>>::ID32: ArrayLength<u8>,
    {
        let buf = self.rx_packetbuffer();

        // Handle packet type
        match buf.ptype {
            HidIoPacketType::Data | HidIoPacketType::NAData => {
                if buf.data.len() < 1 {
                    return Err(CommandError::DataVecTooSmall);
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
                        data.resize_default(buf.data.len() + 5).unwrap();

                        // Serialize buffer
                        self.send_buffer(&mut data, &mut buf)
                    }
                    Err(_nak) => self.byte_nak(property as u8),
                }
            }
            HidIoPacketType::ACK => {
                if buf.data.len() < 1 {
                    return Err(CommandError::DataVecTooSmall);
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
                    return Err(CommandError::DataVecTooSmall);
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
}

// ----- Tests -----

#[cfg(test)]
mod test {
    use super::*;
    use flexi_logger::Logger;
    use heapless::consts::{U100, U2, U3, U64, U8};

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
            for entry in &ENTRIES {
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
            for entry in &ENTRIES {
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
    const ENTRIES: [H0001TestEntry; 13] = [
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
        for entry in &ENTRIES {
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

    /*
    #[test]
    fn h0002_test_test() {
        setup_logging_lite().ok();

        // TODO
        assert!(false, "BLA");
    }

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

    #[test]
    fn hFFFF_invalid_test() {
        setup_logging_lite().ok();

        // TODO
        assert!(false, "BLA");
    }
    */
}
