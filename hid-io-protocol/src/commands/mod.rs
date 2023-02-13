/* Copyright (C) 2020-2023 by Jacob Alexander
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

#[cfg(feature = "defmt")]
use defmt::trace;
#[cfg(not(feature = "defmt"))]
use log::trace;

// ----- Modules -----

mod test;

// ----- Macros -----

// ----- Enumerations -----

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum CommandError {
    BufferInUse,
    BufferNotReady,
    CallbackFailed,
    DataVecNoData,
    DataVecTooSmall,
    IdNotImplemented(HidIoCommandId, HidIoPacketType),
    IdNotMatched(HidIoCommandId),
    IdNotSupported(HidIoCommandId),
    IdVecTooSmall,
    InvalidCStr,
    InvalidId(u32),
    InvalidPacketBufferType(HidIoPacketType),
    InvalidProperty8(u8),
    InvalidRxMessage(HidIoPacketType),
    InvalidUtf8(Utf8Error),
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

// ----- Defmt Wrappers -----

/// Defmt wrapper for core::str::Utf8Error
#[derive(Debug)]
pub struct Utf8Error {
    pub inner: core::str::Utf8Error,
}

impl Utf8Error {
    pub fn new(e: core::str::Utf8Error) -> Self {
        Utf8Error { inner: e }
    }
}

#[cfg(feature = "defmt")]
impl defmt::Format for Utf8Error {
    fn format(&self, fmt: defmt::Formatter) {
        if let Some(error_len) = self.inner.error_len() {
            defmt::write!(
                fmt,
                "invalid utf-8 sequence of {} bytes from index {}",
                error_len,
                self.inner.valid_up_to()
            )
        } else {
            defmt::write!(
                fmt,
                "incomplete utf-8 byte sequence from index {}",
                self.inner.valid_up_to()
            )
        }
    }
}

#[cfg(not(feature = "defmt"))]
impl fmt::Display for Utf8Error {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(error_len) = self.inner.error_len() {
            write!(
                fmt,
                "invalid utf-8 sequence of {} bytes from index {}",
                error_len,
                self.inner.valid_up_to()
            )
        } else {
            write!(
                fmt,
                "incomplete utf-8 byte sequence from index {}",
                self.inner.valid_up_to()
            )
        }
    }
}

// ----- Command Structs -----

/// Supported Ids
pub mod h0000 {
    use super::super::HidIoCommandId;
    use heapless::Vec;

    #[derive(Clone, Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct Cmd {}

    #[derive(Clone, Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct Ack<const ID: usize> {
        pub ids: Vec<HidIoCommandId, ID>,
    }

    #[derive(Clone, Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct Nak {}
}

/// Info Query
pub mod h0001 {
    use heapless::String;
    use num_enum::TryFromPrimitive;

    #[repr(u8)]
    #[derive(PartialEq, Eq, Clone, Copy, Debug, TryFromPrimitive)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum Property {
        Unknown = 0x00,
        MajorVersion = 0x01,
        MinorVersion = 0x02,
        PatchVersion = 0x03,
        DeviceName = 0x04,
        DeviceSerialNumber = 0x05,
        DeviceVersion = 0x06,
        DeviceMcu = 0x07,
        FirmwareName = 0x08,
        FirmwareVersion = 0x09,
        DeviceVendor = 0x0A,
        OsType = 0x0B,
        OsVersion = 0x0C,
        HostSoftwareName = 0x0D,
    }

    #[repr(u8)]
    #[derive(PartialEq, Eq, Clone, Copy, Debug, TryFromPrimitive)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum OsType {
        Unknown = 0x00,
        Windows = 0x01,
        Linux = 0x02,
        Android = 0x03,
        MacOs = 0x04,
        Ios = 0x05,
        ChromeOs = 0x06,
        FreeBsd = 0x07,
        OpenBsd = 0x08,
        NetBsd = 0x09,
    }

    #[derive(Clone, Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct Cmd {
        pub property: Property,
    }

    #[derive(Clone, Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct Ack<const S: usize> {
        pub property: Property,

        /// OS Type field
        pub os: OsType,

        /// Number is set when the given property specifies a number
        pub number: u16,

        /// String is set when the given property specifies a string
        /// Should be 1 byte less than the max hidio data buffer size
        pub string: String<S>,
    }

    #[derive(Clone, Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct Nak {
        pub property: Property,
    }
}

/// Test Message
pub mod h0002 {
    use heapless::Vec;

    #[derive(Clone, Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct Cmd<const D: usize> {
        pub data: Vec<u8, D>,
    }

    #[derive(Clone, Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct Ack<const D: usize> {
        pub data: Vec<u8, D>,
    }

    #[derive(Clone, Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct Nak {}
}

/// Reset HID-IO
pub mod h0003 {
    #[derive(Clone, Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct Cmd {}

    #[derive(Clone, Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct Ack {}

    #[derive(Clone, Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct Nak {}
}

/// Get Properties
pub mod h0010 {
    use heapless::{String, Vec};
    use num_enum::TryFromPrimitive;

    #[repr(u8)]
    #[derive(PartialEq, Eq, Clone, Copy, Debug, TryFromPrimitive)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum Command {
        ListFields = 0x00,
        GetFieldName = 0x01,
        GetFieldValue = 0x02,
        Unknown = 0xFF,
    }

    #[derive(Clone, Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct Cmd {
        pub command: Command,

        /// 8-bit field id
        /// Ignored by ListFields
        pub field: u8,
    }

    #[derive(Clone, Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct Ack<const S: usize> {
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
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
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
    #[derive(PartialEq, Eq, Clone, Copy, Debug, TryFromPrimitive)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum Error {
        NotSupported = 0x00,
        Disabled = 0x01,
    }

    #[derive(Clone, Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct Cmd {}

    #[derive(Clone, Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct Ack {
        pub scancode: u16,
    }

    #[derive(Clone, Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct Nak {
        pub error: Error,
    }
}

/// UTF-8 Character Stream
pub mod h0017 {
    use heapless::String;

    #[derive(Clone, Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct Cmd<const S: usize> {
        pub string: String<S>,
    }

    #[derive(Clone, Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct Ack {}

    #[derive(Clone, Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct Nak {}
}

/// UTF-8 State
pub mod h0018 {
    use heapless::String;

    #[derive(Clone, Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct Cmd<const S: usize> {
        pub symbols: String<S>,
    }

    #[derive(Clone, Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct Ack {}

    #[derive(Clone, Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
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
    #[derive(PartialEq, Eq, Clone, Copy, Debug, TryFromPrimitive)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum Error {
        NotSupported = 0x00,
        Disabled = 0x01,
        NotReady = 0x02,
    }

    #[derive(Clone, Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct Cmd {}

    #[derive(Clone, Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct Ack {}

    #[derive(Clone, Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct Nak {
        pub error: Error,
    }
}

/// KLL Trigger State
pub mod h0020 {
    pub struct Cmd {
        pub event: kll_core::TriggerEvent,
    }
    pub struct Ack {}
    pub struct Nak {}
}

/// Pixel Settings
/// Higher level LED operations and access to LED controller functionality
pub mod h0021 {
    use num_enum::TryFromPrimitive;

    #[repr(u16)]
    #[derive(PartialEq, Eq, Clone, Copy, Debug, TryFromPrimitive)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum Command {
        Control = 0x0001,
        Reset = 0x0002,
        Clear = 0x0003,
        Frame = 0x0004,
        InvalidCommand = 0xFFFF,
    }

    #[derive(Clone, Copy)]
    pub union Argument {
        pub raw: u16,
        pub control: args::Control,
        pub reset: args::Reset,
        pub clear: args::Clear,
        pub frame: args::Frame,
    }

    impl core::fmt::Debug for Argument {
        fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
            write!(f, "{}", unsafe { self.raw })
        }
    }

    #[cfg(feature = "defmt")]
    impl defmt::Format for Argument {
        fn format(&self, fmt: defmt::Formatter) {
            defmt::write!(fmt, "{}", unsafe { self.raw })
        }
    }

    pub mod args {
        use num_enum::TryFromPrimitive;

        #[repr(u16)]
        #[derive(PartialEq, Eq, Clone, Copy, Debug, TryFromPrimitive)]
        #[cfg_attr(feature = "defmt", derive(defmt::Format))]
        pub enum Control {
            Disable = 0x0000,
            EnableStart = 0x0001,
            EnablePause = 0x0002,
        }

        #[repr(u16)]
        #[derive(PartialEq, Eq, Clone, Copy, Debug, TryFromPrimitive)]
        #[cfg_attr(feature = "defmt", derive(defmt::Format))]
        pub enum Reset {
            SoftReset = 0x0000,
            HardReset = 0x0001,
        }

        #[repr(u16)]
        #[derive(PartialEq, Eq, Clone, Copy, Debug, TryFromPrimitive)]
        #[cfg_attr(feature = "defmt", derive(defmt::Format))]
        pub enum Clear {
            Clear = 0x0000,
        }

        #[repr(u16)]
        #[derive(PartialEq, Eq, Clone, Copy, Debug, TryFromPrimitive)]
        #[cfg_attr(feature = "defmt", derive(defmt::Format))]
        pub enum Frame {
            NextFrame = 0x0000,
        }
    }

    #[derive(Clone, Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct Cmd {
        pub command: Command,
        pub argument: Argument,
    }

    #[derive(Clone, Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct Ack {}

    #[derive(Clone, Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
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

/// Direct Channel Set
/// Control the buffer directly (device configuration dependent)
pub mod h0026 {
    use heapless::Vec;

    #[derive(Clone, Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct Cmd<const D: usize> {
        pub start_address: u16,
        pub data: Vec<u8, D>,
    }

    #[derive(Clone, Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct Ack {}

    #[derive(Clone, Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct Nak {}
}

/// Open URL
pub mod h0030 {
    use heapless::String;

    #[derive(Clone, Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct Cmd<const S: usize> {
        pub url: String<S>,
    }

    #[derive(Clone, Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct Ack {}

    #[derive(Clone, Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct Nak {}
}

/// Terminal Command
pub mod h0031 {
    use heapless::String;

    #[derive(Clone, Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct Cmd<const S: usize> {
        pub command: String<S>,
    }

    #[derive(Clone, Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct Ack {}

    #[derive(Clone, Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
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
    use heapless::String;

    #[derive(Clone, Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct Cmd<const S: usize> {
        pub output: String<S>,
    }

    #[derive(Clone, Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct Ack {}

    #[derive(Clone, Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
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
    use num_enum::TryFromPrimitive;

    #[repr(u16)]
    #[derive(PartialEq, Eq, Clone, Copy, Debug, TryFromPrimitive)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum Command {
        TestCommand = 0x0000,
        LedTestSequence = 0x0001,
        LedCycleKeypressTest = 0x0002,
        HallEffectSensorTest = 0x0003,
        InvalidCommand = 0x9999,
    }

    pub mod args {
        use num_enum::TryFromPrimitive;

        #[repr(u16)]
        #[derive(PartialEq, Eq, Clone, Copy, Debug, TryFromPrimitive)]
        #[cfg_attr(feature = "defmt", derive(defmt::Format))]
        pub enum LedTestSequence {
            Disable = 0x0000,
            Enable = 0x0001,
            ActivateLedShortTest = 0x0002,
            ActivateLedOpenCircuitTest = 0x0003,
        }

        #[repr(u16)]
        #[derive(PartialEq, Eq, Clone, Copy, Debug, TryFromPrimitive)]
        #[cfg_attr(feature = "defmt", derive(defmt::Format))]
        pub enum LedCycleKeypressTest {
            Disable = 0x0000,
            Enable = 0x0001,
        }

        #[repr(u16)]
        #[derive(PartialEq, Eq, Clone, Copy, Debug, TryFromPrimitive)]
        #[cfg_attr(feature = "defmt", derive(defmt::Format))]
        pub enum HallEffectSensorTest {
            DisableAll = 0x0000,
            PassFailTestToggle = 0x0001,
            LevelCheckToggle = 0x0002,
            AdcTestModeToggle = 0x0003,
            LevelCheckColumn1Toggle = 0x0011,
            LevelCheckColumn2Toggle = 0x0012,
            LevelCheckColumn3Toggle = 0x0013,
            LevelCheckColumn4Toggle = 0x0014,
            LevelCheckColumn5Toggle = 0x0015,
            LevelCheckColumn6Toggle = 0x0016,
            LevelCheckColumn7Toggle = 0x0017,
            LevelCheckColumn8Toggle = 0x0018,
            LevelCheckColumn9Toggle = 0x0019,
            LevelCheckColumn10Toggle = 0x001A,
            LevelCheckColumn11Toggle = 0x001B,
            LevelCheckColumn12Toggle = 0x001C,
            LevelCheckColumn13Toggle = 0x001D,
            LevelCheckColumn14Toggle = 0x001E,
            LevelCheckColumn15Toggle = 0x001F,
            LevelCheckColumn16Toggle = 0x0020,
            LevelCheckColumn17Toggle = 0x0021,
            LevelCheckColumn18Toggle = 0x0022,
            LevelCheckColumn19Toggle = 0x0023,
            LevelCheckColumn20Toggle = 0x0024,
            LevelCheckColumn21Toggle = 0x0025,
            LevelCheckColumn22Toggle = 0x0026,
            ModeSetNormal = 0x0100,
            ModeSetLowLatency = 0x0101,
            ModeSetTest = 0x0102,
        }
    }

    #[derive(Clone, Copy)]
    pub union Argument {
        pub raw: u16,
        pub led_test_sequence: args::LedTestSequence,
        pub led_cycle_keypress_test: args::LedCycleKeypressTest,
        pub hall_effect_sensor_test: args::HallEffectSensorTest,
    }

    impl core::fmt::Debug for Argument {
        fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
            write!(f, "{}", unsafe { self.raw })
        }
    }

    #[cfg(feature = "defmt")]
    impl defmt::Format for Argument {
        fn format(&self, fmt: defmt::Formatter) {
            defmt::write!(fmt, "{}", unsafe { self.raw })
        }
    }

    #[derive(Clone, Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct Cmd {
        pub command: Command,
        pub argument: Argument,
    }

    #[derive(Clone, Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct Ack {}

    #[derive(Clone, Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct Nak {}
}

/// Manufacturing Test Result
pub mod h0051 {
    use heapless::Vec;
    use num_enum::TryFromPrimitive;

    #[repr(u16)]
    #[derive(PartialEq, Eq, Clone, Copy, Debug, TryFromPrimitive)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum Command {
        TestCommand = 0x0000,
        LedTestSequence = 0x0001,
        LedCycleKeypressTest = 0x0002,
        HallEffectSensorTest = 0x0003,
        InvalidCommand = 0x9999,
    }

    pub mod args {
        use num_enum::TryFromPrimitive;

        #[repr(u16)]
        #[derive(PartialEq, Eq, Clone, Copy, Debug, TryFromPrimitive)]
        #[cfg_attr(feature = "defmt", derive(defmt::Format))]
        pub enum LedTestSequence {
            LedShortTest = 0x0002,
            LedOpenCircuitTest = 0x0003,
        }

        #[repr(u16)]
        #[derive(PartialEq, Eq, Clone, Copy, Debug, TryFromPrimitive)]
        #[cfg_attr(feature = "defmt", derive(defmt::Format))]
        pub enum LedCycleKeypressTest {
            Enable = 0x0001,
        }

        #[repr(u16)]
        #[derive(PartialEq, Eq, Clone, Copy, Debug, TryFromPrimitive)]
        #[cfg_attr(feature = "defmt", derive(defmt::Format))]
        pub enum HallEffectSensorTest {
            PassFailTest = 0x0001,
            LevelCheck = 0x0002,
        }
    }

    #[derive(Clone, Copy)]
    pub union Argument {
        pub raw: u16,
        pub led_test_sequence: args::LedTestSequence,
        pub led_cycle_keypress_test: args::LedCycleKeypressTest,
        pub hall_effect_sensor_test: args::HallEffectSensorTest,
    }

    impl core::fmt::Debug for Argument {
        fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
            write!(f, "{}", unsafe { self.raw })
        }
    }

    #[cfg(feature = "defmt")]
    impl defmt::Format for Argument {
        fn format(&self, fmt: defmt::Formatter) {
            defmt::write!(fmt, "{}", unsafe { self.raw })
        }
    }

    #[derive(Clone, Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct Cmd<const D: usize> {
        pub command: Command,
        pub argument: Argument,
        pub data: Vec<u8, D>,
    }

    #[derive(Clone, Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct Ack {}

    #[derive(Clone, Debug)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub struct Nak {}
}

// ----- Traits -----

/// HID-IO Command Interface
/// H - Max data payload length (HidIoPacketBuffer)
/// HSUB1, HSUB2, HSUB4 - Due to current limitations of const generics (missing
/// const_evaluatable_checked), H - 1, H - 2 and H - 4 must be defined at the top-level.
/// ID - Max number of HidIoCommandIds
pub trait Commands<
    const H: usize,
    const HSUB1: usize,
    const HSUB2: usize,
    const HSUB4: usize,
    const ID: usize,
>
{
    /// Given a HidIoPacketBuffer serialize (and resulting send bytes)
    fn tx_packetbuffer_send(&mut self, buf: &mut HidIoPacketBuffer<H>) -> Result<(), CommandError>;

    /// Check if id is valid for this interface
    /// (By default support all ids)
    fn supported_id(&self, _id: HidIoCommandId) -> bool {
        true
    }

    /// Default packet chunk
    /// (Usual chunk sizes are 63 or 64)
    fn default_packet_chunk(&self) -> u32 {
        64
    }

    /// Simple empty ack
    fn empty_ack(&mut self, id: HidIoCommandId) -> Result<(), CommandError> {
        // Build empty Ack
        self.tx_packetbuffer_send(&mut HidIoPacketBuffer {
            // Data packet
            ptype: HidIoPacketType::Ack,
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
    fn empty_nak(&mut self, id: HidIoCommandId) -> Result<(), CommandError> {
        // Build empty Nak
        self.tx_packetbuffer_send(&mut HidIoPacketBuffer {
            // Data packet
            ptype: HidIoPacketType::Nak,
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
    fn byte_ack(&mut self, id: HidIoCommandId, byte: u8) -> Result<(), CommandError> {
        // Build Ack
        self.tx_packetbuffer_send(&mut HidIoPacketBuffer {
            // Data packet
            ptype: HidIoPacketType::Ack,
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
    fn byte_nak(&mut self, id: HidIoCommandId, byte: u8) -> Result<(), CommandError> {
        // Build Nak
        self.tx_packetbuffer_send(&mut HidIoPacketBuffer {
            // Data packet
            ptype: HidIoPacketType::Nak,
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
    fn short_ack(&mut self, id: HidIoCommandId, val: u16) -> Result<(), CommandError> {
        // Build Ack
        self.tx_packetbuffer_send(&mut HidIoPacketBuffer {
            // Data packet
            ptype: HidIoPacketType::Ack,
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
    fn short_nak(&mut self, id: HidIoCommandId, val: u16) -> Result<(), CommandError> {
        // Build Nak
        self.tx_packetbuffer_send(&mut HidIoPacketBuffer {
            // Data packet
            ptype: HidIoPacketType::Nak,
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
    fn rx_message_handling(&mut self, buf: HidIoPacketBuffer<H>) -> Result<(), CommandError> {
        // Make sure we're processing a supported id
        if !self.supported_id(buf.id) {
            self.empty_nak(buf.id)?;
            return Err(CommandError::IdNotSupported(buf.id));
        }

        // Check for invalid packet types
        match buf.ptype {
            HidIoPacketType::Data | HidIoPacketType::NaData => {}
            HidIoPacketType::Ack => {}
            HidIoPacketType::Nak => {}
            _ => {
                return Err(CommandError::InvalidRxMessage(buf.ptype));
            }
        }

        // Match id
        trace!("rx_message_handling: {:?}", buf);
        match buf.id {
            HidIoCommandId::SupportedIds => self.h0000_supported_ids_handler(buf),
            HidIoCommandId::GetInfo => self.h0001_info_handler(buf),
            HidIoCommandId::TestPacket => self.h0002_test_handler(buf),
            HidIoCommandId::ResetHidIo => self.h0003_resethidio_handler(buf),
            HidIoCommandId::FlashMode => self.h0016_flashmode_handler(buf),
            HidIoCommandId::UnicodeText => self.h0017_unicodetext_handler(buf),
            HidIoCommandId::UnicodeState => self.h0018_unicodestate_handler(buf),
            HidIoCommandId::SleepMode => self.h001a_sleepmode_handler(buf),
            HidIoCommandId::PixelSetting => self.h0021_pixelsetting_handler(buf),
            HidIoCommandId::DirectSet => self.h0026_directset_handler(buf),
            HidIoCommandId::OpenUrl => self.h0030_openurl_handler(buf),
            HidIoCommandId::TerminalCmd => self.h0031_terminalcmd_handler(buf),
            HidIoCommandId::TerminalOut => self.h0034_terminalout_handler(buf),
            HidIoCommandId::ManufacturingTest => self.h0050_manufacturing_handler(buf),
            HidIoCommandId::ManufacturingResult => self.h0051_manufacturingres_handler(buf),
            _ => Err(CommandError::IdNotMatched(buf.id)),
        }
    }

    fn h0000_supported_ids(&mut self, _data: h0000::Cmd) -> Result<(), CommandError> {
        self.tx_packetbuffer_send(&mut HidIoPacketBuffer {
            // Test packet id
            id: HidIoCommandId::SupportedIds,
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
            HidIoCommandId::SupportedIds,
            HidIoPacketType::Ack,
        ))
    }
    fn h0000_supported_ids_nak(&mut self, _data: h0000::Nak) -> Result<(), CommandError> {
        Err(CommandError::IdNotImplemented(
            HidIoCommandId::SupportedIds,
            HidIoPacketType::Nak,
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
                        // Build Ack
                        let mut buf = HidIoPacketBuffer {
                            // Data packet
                            ptype: HidIoPacketType::Ack,
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
            HidIoPacketType::NaData => Err(CommandError::InvalidPacketBufferType(buf.ptype)),
            HidIoPacketType::Ack => {
                // Retrieve list of ids
                let mut ids: Vec<HidIoCommandId, ID> = Vec::new();
                // Ids are always 16-bit le for this command
                let mut pos = 0;
                while pos <= buf.data.len() - 2 {
                    let slice = &buf.data[pos..pos + 2];
                    let idnum = u16::from_le_bytes(slice.try_into().unwrap()) as u32;
                    // Make sure this is a valid id
                    let id = match HidIoCommandId::try_from(idnum) {
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
            HidIoPacketType::Nak => self.h0000_supported_ids_nak(h0000::Nak {}),
            _ => Ok(()),
        }
    }

    fn h0001_info(&mut self, data: h0001::Cmd) -> Result<(), CommandError> {
        // Create appropriately sized buffer
        let mut buf = HidIoPacketBuffer {
            // Test packet id
            id: HidIoCommandId::GetInfo,
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
        trace!("h0001_info: {:?} - {:?}", data, buf);

        self.tx_packetbuffer_send(&mut buf)
    }
    fn h0001_info_cmd(&mut self, _data: h0001::Cmd) -> Result<h0001::Ack<HSUB1>, h0001::Nak> {
        Err(h0001::Nak {
            property: h0001::Property::Unknown,
        })
    }
    fn h0001_info_ack(&mut self, _data: h0001::Ack<HSUB1>) -> Result<(), CommandError> {
        Err(CommandError::IdNotImplemented(
            HidIoCommandId::GetInfo,
            HidIoPacketType::Ack,
        ))
    }
    fn h0001_info_nak(&mut self, _data: h0001::Nak) -> Result<(), CommandError> {
        Err(CommandError::IdNotImplemented(
            HidIoCommandId::GetInfo,
            HidIoPacketType::Nak,
        ))
    }
    fn h0001_info_handler(&mut self, buf: HidIoPacketBuffer<H>) -> Result<(), CommandError> {
        // Handle packet type
        match buf.ptype {
            HidIoPacketType::Data => {
                if buf.data.is_empty() {
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
                        // Build Ack
                        let mut buf = HidIoPacketBuffer {
                            // Data packet
                            ptype: HidIoPacketType::Ack,
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
            HidIoPacketType::NaData => Err(CommandError::InvalidPacketBufferType(buf.ptype)),
            HidIoPacketType::Ack => {
                if buf.data.is_empty() {
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
                    os: h0001::OsType::Unknown,
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
                        ack.os = match h0001::OsType::try_from(typenum) {
                            Ok(ostype) => ostype,
                            Err(_) => {
                                return Err(CommandError::InvalidProperty8(typenum));
                            }
                        };
                    }
                    // Handle ascii values
                    _ => {
                        ack.string
                            .push_str(match core::str::from_utf8(&buf.data[1..]) {
                                Ok(s) => s,
                                Err(e) => {
                                    return Err(CommandError::InvalidUtf8(Utf8Error::new(e)));
                                }
                            })
                            .unwrap();
                    }
                }

                self.h0001_info_ack(ack)
            }
            HidIoPacketType::Nak => {
                if buf.data.is_empty() {
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
            id: HidIoCommandId::TestPacket,
            // Detect max size
            max_len: self.default_packet_chunk(),
            // Use defaults for other fields
            ..Default::default()
        };

        // Set NA (no-ack)
        if na {
            buf.ptype = HidIoPacketType::NaData;
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
            HidIoCommandId::TestPacket,
            HidIoPacketType::NaData,
        ))
    }
    fn h0002_test_ack(&mut self, _data: h0002::Ack<H>) -> Result<(), CommandError> {
        Err(CommandError::IdNotImplemented(
            HidIoCommandId::TestPacket,
            HidIoPacketType::Ack,
        ))
    }
    fn h0002_test_nak(&mut self, _data: h0002::Nak) -> Result<(), CommandError> {
        Err(CommandError::IdNotImplemented(
            HidIoCommandId::TestPacket,
            HidIoPacketType::Nak,
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
                        // Build Ack (max test data size)
                        let mut buf = HidIoPacketBuffer {
                            // Data packet
                            ptype: HidIoPacketType::Ack,
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
            HidIoPacketType::NaData => {
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
            HidIoPacketType::Ack => {
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
            HidIoPacketType::Nak => self.h0002_test_nak(h0002::Nak {}),
            _ => Ok(()),
        }
    }

    fn h0003_resethidio(&mut self, _data: h0003::Cmd) -> Result<(), CommandError> {
        self.tx_packetbuffer_send(&mut HidIoPacketBuffer {
            // Test packet id
            id: HidIoCommandId::ResetHidIo,
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
            HidIoCommandId::ResetHidIo,
            HidIoPacketType::Ack,
        ))
    }
    fn h0003_resethidio_nak(&mut self, _data: h0003::Nak) -> Result<(), CommandError> {
        Err(CommandError::IdNotImplemented(
            HidIoCommandId::ResetHidIo,
            HidIoPacketType::Nak,
        ))
    }
    fn h0003_resethidio_handler(&mut self, buf: HidIoPacketBuffer<H>) -> Result<(), CommandError> {
        // Handle packet type
        match buf.ptype {
            HidIoPacketType::Data => match self.h0003_resethidio_cmd(h0003::Cmd {}) {
                Ok(_ack) => self.empty_ack(buf.id),
                Err(_nak) => self.empty_nak(buf.id),
            },
            HidIoPacketType::NaData => Err(CommandError::InvalidPacketBufferType(buf.ptype)),
            HidIoPacketType::Ack => self.h0003_resethidio_ack(h0003::Ack {}),
            HidIoPacketType::Nak => self.h0003_resethidio_nak(h0003::Nak {}),
            _ => Ok(()),
        }
    }

    fn h0016_flashmode(&mut self, _data: h0016::Cmd) -> Result<(), CommandError> {
        self.tx_packetbuffer_send(&mut HidIoPacketBuffer {
            // Test packet id
            id: HidIoCommandId::FlashMode,
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
            HidIoCommandId::FlashMode,
            HidIoPacketType::Ack,
        ))
    }
    fn h0016_flashmode_nak(&mut self, _data: h0016::Nak) -> Result<(), CommandError> {
        Err(CommandError::IdNotImplemented(
            HidIoCommandId::FlashMode,
            HidIoPacketType::Nak,
        ))
    }
    fn h0016_flashmode_handler(&mut self, buf: HidIoPacketBuffer<H>) -> Result<(), CommandError> {
        // Handle packet type
        match buf.ptype {
            HidIoPacketType::Data => match self.h0016_flashmode_cmd(h0016::Cmd {}) {
                Ok(ack) => self.short_ack(buf.id, ack.scancode),
                Err(nak) => self.byte_nak(buf.id, nak.error as u8),
            },
            HidIoPacketType::NaData => Err(CommandError::InvalidPacketBufferType(buf.ptype)),
            HidIoPacketType::Ack => {
                if buf.data.len() < 2 {
                    return Err(CommandError::DataVecNoData);
                }

                let scancode = u16::from_le_bytes(buf.data[0..2].try_into().unwrap());
                self.h0016_flashmode_ack(h0016::Ack { scancode })
            }
            HidIoPacketType::Nak => {
                if buf.data.is_empty() {
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
            id: HidIoCommandId::UnicodeText,
            // Detect max size
            max_len: self.default_packet_chunk(),
            // Use defaults for other fields
            ..Default::default()
        };

        // Set NA (no-ack)
        if na {
            buf.ptype = HidIoPacketType::NaData;
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
            HidIoCommandId::UnicodeText,
            HidIoPacketType::NaData,
        ))
    }
    fn h0017_unicodetext_ack(&mut self, _data: h0017::Ack) -> Result<(), CommandError> {
        Err(CommandError::IdNotImplemented(
            HidIoCommandId::UnicodeText,
            HidIoPacketType::Ack,
        ))
    }
    fn h0017_unicodetext_nak(&mut self, _data: h0017::Nak) -> Result<(), CommandError> {
        Err(CommandError::IdNotImplemented(
            HidIoCommandId::UnicodeText,
            HidIoPacketType::Nak,
        ))
    }
    fn h0017_unicodetext_handler(&mut self, buf: HidIoPacketBuffer<H>) -> Result<(), CommandError> {
        // Handle packet type
        match buf.ptype {
            HidIoPacketType::Data => {
                // Copy data into struct
                let mut cmd = h0017::Cmd::<H> {
                    string: String::new(),
                };
                cmd.string
                    .push_str(match core::str::from_utf8(&buf.data) {
                        Ok(string) => string,
                        Err(e) => {
                            return Err(CommandError::InvalidUtf8(Utf8Error::new(e)));
                        }
                    })
                    .unwrap();

                match self.h0017_unicodetext_cmd(cmd) {
                    Ok(_ack) => self.empty_ack(buf.id),
                    Err(_nak) => self.empty_nak(buf.id),
                }
            }
            HidIoPacketType::NaData => {
                // Copy data into struct
                let mut cmd = h0017::Cmd::<H> {
                    string: String::new(),
                };
                cmd.string
                    .push_str(match core::str::from_utf8(&buf.data) {
                        Ok(string) => string,
                        Err(e) => {
                            return Err(CommandError::InvalidUtf8(Utf8Error::new(e)));
                        }
                    })
                    .unwrap();

                self.h0017_unicodetext_nacmd(cmd)
            }
            HidIoPacketType::Ack => self.h0017_unicodetext_ack(h0017::Ack {}),
            HidIoPacketType::Nak => self.h0017_unicodetext_nak(h0017::Nak {}),
            _ => Ok(()),
        }
    }

    fn h0018_unicodestate(&mut self, data: h0018::Cmd<H>, na: bool) -> Result<(), CommandError> {
        // Create appropriately sized buffer
        let mut buf = HidIoPacketBuffer {
            // Test packet id
            id: HidIoCommandId::UnicodeState,
            // Detect max size
            max_len: self.default_packet_chunk(),
            // Use defaults for other fields
            ..Default::default()
        };

        // Set NA (no-ack)
        if na {
            buf.ptype = HidIoPacketType::NaData;
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
            HidIoCommandId::UnicodeState,
            HidIoPacketType::NaData,
        ))
    }
    fn h0018_unicodestate_ack(&mut self, _data: h0018::Ack) -> Result<(), CommandError> {
        Err(CommandError::IdNotImplemented(
            HidIoCommandId::UnicodeState,
            HidIoPacketType::Ack,
        ))
    }
    fn h0018_unicodestate_nak(&mut self, _data: h0018::Nak) -> Result<(), CommandError> {
        Err(CommandError::IdNotImplemented(
            HidIoCommandId::UnicodeState,
            HidIoPacketType::Nak,
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
                let mut cmd = h0018::Cmd::<H> {
                    symbols: String::new(),
                };
                cmd.symbols
                    .push_str(match core::str::from_utf8(&buf.data) {
                        Ok(symbols) => symbols,
                        Err(e) => {
                            return Err(CommandError::InvalidUtf8(Utf8Error::new(e)));
                        }
                    })
                    .unwrap();

                match self.h0018_unicodestate_cmd(cmd) {
                    Ok(_ack) => self.empty_ack(buf.id),
                    Err(_nak) => self.empty_nak(buf.id),
                }
            }
            HidIoPacketType::NaData => {
                // Copy data into struct
                let mut cmd = h0018::Cmd::<H> {
                    symbols: String::new(),
                };
                cmd.symbols
                    .push_str(match core::str::from_utf8(&buf.data) {
                        Ok(symbols) => symbols,
                        Err(e) => {
                            return Err(CommandError::InvalidUtf8(Utf8Error::new(e)));
                        }
                    })
                    .unwrap();

                self.h0018_unicodestate_nacmd(cmd)
            }
            HidIoPacketType::Ack => self.h0018_unicodestate_ack(h0018::Ack {}),
            HidIoPacketType::Nak => self.h0018_unicodestate_nak(h0018::Nak {}),
            _ => Ok(()),
        }
    }

    fn h001a_sleepmode(&mut self, _data: h001a::Cmd) -> Result<(), CommandError> {
        self.tx_packetbuffer_send(&mut HidIoPacketBuffer {
            // Test packet id
            id: HidIoCommandId::SleepMode,
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
            HidIoCommandId::SleepMode,
            HidIoPacketType::Ack,
        ))
    }
    fn h001a_sleepmode_nak(&mut self, _data: h001a::Nak) -> Result<(), CommandError> {
        Err(CommandError::IdNotImplemented(
            HidIoCommandId::SleepMode,
            HidIoPacketType::Nak,
        ))
    }
    fn h001a_sleepmode_handler(&mut self, buf: HidIoPacketBuffer<H>) -> Result<(), CommandError> {
        // Handle packet type
        match buf.ptype {
            HidIoPacketType::Data => match self.h001a_sleepmode_cmd(h001a::Cmd {}) {
                Ok(_ack) => self.empty_ack(buf.id),
                Err(nak) => self.byte_nak(buf.id, nak.error as u8),
            },
            HidIoPacketType::NaData => Err(CommandError::InvalidPacketBufferType(buf.ptype)),
            HidIoPacketType::Ack => self.h001a_sleepmode_ack(h001a::Ack {}),
            HidIoPacketType::Nak => {
                if buf.data.is_empty() {
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

    fn h0020_klltrigger(&mut self, data: h0020::Cmd, na: bool) -> Result<(), CommandError> {
        // Create appropriately sized buffer
        let mut buf = HidIoPacketBuffer {
            // KllState id
            id: HidIoCommandId::KllState,
            // Detect max size
            max_len: self.default_packet_chunk(),
            // Use defaults for other fields
            ..Default::default()
        };

        // Set NA (no-ack)
        if na {
            buf.ptype = HidIoPacketType::NaData;
        }

        // Build payload
        if !buf.append_payload(unsafe { data.event.bytes() }) {
            return Err(CommandError::DataVecTooSmall);
        }
        buf.done = true;

        self.tx_packetbuffer_send(&mut buf)
    }
    fn h0020_klltrigger_cmd(&mut self, _data: h0020::Cmd) -> Result<h0020::Ack, h0020::Nak> {
        Err(h0020::Nak {})
    }
    fn h0020_klltrigger_nacmd(&mut self, _data: h0020::Cmd) -> Result<(), CommandError> {
        Err(CommandError::IdNotImplemented(
            HidIoCommandId::KllState,
            HidIoPacketType::NaData,
        ))
    }
    fn h0020_klltrigger_ack(&mut self, _data: h0020::Ack) -> Result<(), CommandError> {
        Err(CommandError::IdNotImplemented(
            HidIoCommandId::KllState,
            HidIoPacketType::Ack,
        ))
    }
    fn h0020_klltrigger_nak(&mut self, _data: h0020::Nak) -> Result<(), CommandError> {
        Err(CommandError::IdNotImplemented(
            HidIoCommandId::KllState,
            HidIoPacketType::Nak,
        ))
    }
    fn h0020_klltrigger_handler(&mut self, buf: HidIoPacketBuffer<H>) -> Result<(), CommandError> {
        // Handle packet type
        match buf.ptype {
            HidIoPacketType::Data => {
                // Copy data into struct
                let cmd = h0020::Cmd {
                    event: unsafe { kll_core::TriggerEvent::from_bytes(&buf.data) },
                };

                match self.h0020_klltrigger_cmd(cmd) {
                    Ok(_ack) => self.empty_ack(buf.id),
                    Err(_nak) => self.empty_nak(buf.id),
                }
            }
            HidIoPacketType::NaData => {
                // Copy data into struct
                let cmd = h0020::Cmd {
                    event: unsafe { kll_core::TriggerEvent::from_bytes(&buf.data) },
                };

                self.h0020_klltrigger_nacmd(cmd)
            }
            HidIoPacketType::Ack => self.h0020_klltrigger_ack(h0020::Ack {}),
            HidIoPacketType::Nak => self.h0020_klltrigger_nak(h0020::Nak {}),
            _ => Ok(()),
        }
    }

    fn h0021_pixelsetting(&mut self, data: h0021::Cmd, na: bool) -> Result<(), CommandError> {
        // Create appropriately sized buffer
        let mut buf = HidIoPacketBuffer {
            // KllState id
            id: HidIoCommandId::PixelSetting,
            // Detect max size
            max_len: self.default_packet_chunk(),
            // Use defaults for other fields
            ..Default::default()
        };

        // Set NA (no-ack)
        if na {
            buf.ptype = HidIoPacketType::NaData;
        }

        // Build payload
        if !buf.append_payload(&(data.command as u16).to_le_bytes()) {
            return Err(CommandError::DataVecTooSmall);
        }
        if !buf.append_payload(unsafe { &data.argument.raw.to_le_bytes() }) {
            return Err(CommandError::DataVecTooSmall);
        }
        buf.done = true;

        self.tx_packetbuffer_send(&mut buf)
    }
    fn h0021_pixelsetting_cmd(&mut self, _data: h0021::Cmd) -> Result<h0021::Ack, h0021::Nak> {
        Err(h0021::Nak {})
    }
    fn h0021_pixelsetting_nacmd(&mut self, _data: h0021::Cmd) -> Result<(), CommandError> {
        Err(CommandError::IdNotImplemented(
            HidIoCommandId::PixelSetting,
            HidIoPacketType::NaData,
        ))
    }
    fn h0021_pixelsetting_ack(&mut self, _data: h0021::Ack) -> Result<(), CommandError> {
        Err(CommandError::IdNotImplemented(
            HidIoCommandId::PixelSetting,
            HidIoPacketType::Ack,
        ))
    }
    fn h0021_pixelsetting_nak(&mut self, _data: h0021::Nak) -> Result<(), CommandError> {
        Err(CommandError::IdNotImplemented(
            HidIoCommandId::PixelSetting,
            HidIoPacketType::Nak,
        ))
    }
    fn h0021_pixelsetting_handler(
        &mut self,
        buf: HidIoPacketBuffer<H>,
    ) -> Result<(), CommandError> {
        // Handle packet type
        match buf.ptype {
            HidIoPacketType::Data => {
                // Copy data into struct
                let cmd = h0021::Cmd {
                    command: h0021::Command::try_from(u16::from_le_bytes(
                        buf.data[0..2].try_into().unwrap(),
                    ))
                    .unwrap(),
                    argument: h0021::Argument {
                        raw: u16::from_le_bytes(buf.data[2..4].try_into().unwrap()),
                    },
                };

                match self.h0021_pixelsetting_cmd(cmd) {
                    Ok(_ack) => self.empty_ack(buf.id),
                    Err(_nak) => self.empty_nak(buf.id),
                }
            }
            HidIoPacketType::NaData => {
                // Copy data into struct
                let cmd = h0021::Cmd {
                    command: h0021::Command::try_from(u16::from_le_bytes(
                        buf.data[0..2].try_into().unwrap(),
                    ))
                    .unwrap(),
                    argument: h0021::Argument {
                        raw: u16::from_le_bytes(buf.data[2..4].try_into().unwrap()),
                    },
                };

                self.h0021_pixelsetting_nacmd(cmd)
            }
            HidIoPacketType::Ack => self.h0021_pixelsetting_ack(h0021::Ack {}),
            HidIoPacketType::Nak => self.h0021_pixelsetting_nak(h0021::Nak {}),
            _ => Ok(()),
        }
    }

    fn h0026_directset(&mut self, data: h0026::Cmd<HSUB2>, na: bool) -> Result<(), CommandError> {
        // Create appropriately sized buffer
        let mut buf = HidIoPacketBuffer {
            // KllState id
            id: HidIoCommandId::DirectSet,
            // Detect max size
            max_len: self.default_packet_chunk(),
            // Use defaults for other fields
            ..Default::default()
        };

        // Set NA (no-ack)
        if na {
            buf.ptype = HidIoPacketType::NaData;
        }

        // Build payload
        if !buf.append_payload(&data.start_address.to_le_bytes()) {
            return Err(CommandError::DataVecTooSmall);
        }
        if !buf.append_payload(&data.data) {
            return Err(CommandError::DataVecTooSmall);
        }
        buf.done = true;

        self.tx_packetbuffer_send(&mut buf)
    }
    fn h0026_directset_cmd(&mut self, _data: h0026::Cmd<HSUB2>) -> Result<h0026::Ack, h0026::Nak> {
        Err(h0026::Nak {})
    }
    fn h0026_directset_nacmd(&mut self, _data: h0026::Cmd<HSUB2>) -> Result<(), CommandError> {
        Err(CommandError::IdNotImplemented(
            HidIoCommandId::DirectSet,
            HidIoPacketType::NaData,
        ))
    }
    fn h0026_directset_ack(&mut self, _data: h0026::Ack) -> Result<(), CommandError> {
        Err(CommandError::IdNotImplemented(
            HidIoCommandId::DirectSet,
            HidIoPacketType::Ack,
        ))
    }
    fn h0026_directset_nak(&mut self, _data: h0026::Nak) -> Result<(), CommandError> {
        Err(CommandError::IdNotImplemented(
            HidIoCommandId::DirectSet,
            HidIoPacketType::Nak,
        ))
    }
    fn h0026_directset_handler(&mut self, buf: HidIoPacketBuffer<H>) -> Result<(), CommandError> {
        // Handle packet type
        match buf.ptype {
            HidIoPacketType::Data => {
                // Copy data into struct
                let cmd = h0026::Cmd::<HSUB2> {
                    start_address: u16::from_le_bytes([buf.data[0], buf.data[1]]),
                    data: match Vec::from_slice(&buf.data[2..buf.data.len()]) {
                        Ok(data) => data,
                        Err(_) => {
                            return Err(CommandError::DataVecTooSmall);
                        }
                    },
                };

                match self.h0026_directset_cmd(cmd) {
                    Ok(_ack) => self.empty_ack(buf.id),
                    Err(_nak) => self.empty_nak(buf.id),
                }
            }
            HidIoPacketType::NaData => {
                // Copy data into struct
                let cmd = h0026::Cmd::<HSUB2> {
                    start_address: u16::from_le_bytes([buf.data[0], buf.data[1]]),
                    data: match Vec::from_slice(&buf.data[2..buf.data.len()]) {
                        Ok(data) => data,
                        Err(_) => {
                            return Err(CommandError::DataVecTooSmall);
                        }
                    },
                };

                self.h0026_directset_nacmd(cmd)
            }
            HidIoPacketType::Ack => self.h0026_directset_ack(h0026::Ack {}),
            HidIoPacketType::Nak => self.h0026_directset_nak(h0026::Nak {}),
            _ => Ok(()),
        }
    }

    fn h0030_openurl(&mut self, data: h0030::Cmd<H>) -> Result<(), CommandError> {
        // Create appropriately sized buffer
        let mut buf = HidIoPacketBuffer {
            // Test packet id
            id: HidIoCommandId::OpenUrl,
            // Detect max size
            max_len: self.default_packet_chunk(),
            // Use defaults for other fields
            ..Default::default()
        };

        // Build payload
        if !buf.append_payload(data.url.as_bytes()) {
            return Err(CommandError::DataVecTooSmall);
        }
        buf.done = true;

        self.tx_packetbuffer_send(&mut buf)
    }
    fn h0030_openurl_cmd(&mut self, _data: h0030::Cmd<H>) -> Result<h0030::Ack, h0030::Nak> {
        Err(h0030::Nak {})
    }
    fn h0030_openurl_nacmd(&mut self, _data: h0030::Cmd<H>) -> Result<(), CommandError> {
        Err(CommandError::IdNotImplemented(
            HidIoCommandId::OpenUrl,
            HidIoPacketType::NaData,
        ))
    }
    fn h0030_openurl_ack(&mut self, _data: h0030::Ack) -> Result<(), CommandError> {
        Err(CommandError::IdNotImplemented(
            HidIoCommandId::OpenUrl,
            HidIoPacketType::Ack,
        ))
    }
    fn h0030_openurl_nak(&mut self, _data: h0030::Nak) -> Result<(), CommandError> {
        Err(CommandError::IdNotImplemented(
            HidIoCommandId::OpenUrl,
            HidIoPacketType::Nak,
        ))
    }
    fn h0030_openurl_handler(&mut self, buf: HidIoPacketBuffer<H>) -> Result<(), CommandError> {
        // Handle packet type
        match buf.ptype {
            HidIoPacketType::Data => {
                // Copy data into struct
                let mut cmd = h0030::Cmd::<H> { url: String::new() };
                cmd.url
                    .push_str(match core::str::from_utf8(&buf.data) {
                        Ok(url) => url,
                        Err(e) => {
                            return Err(CommandError::InvalidUtf8(Utf8Error::new(e)));
                        }
                    })
                    .unwrap();

                match self.h0030_openurl_cmd(cmd) {
                    Ok(_ack) => self.empty_ack(buf.id),
                    Err(_nak) => self.empty_nak(buf.id),
                }
            }
            HidIoPacketType::NaData => Err(CommandError::InvalidPacketBufferType(buf.ptype)),
            HidIoPacketType::Ack => self.h0030_openurl_ack(h0030::Ack {}),
            HidIoPacketType::Nak => self.h0030_openurl_nak(h0030::Nak {}),
            _ => Ok(()),
        }
    }

    fn h0031_terminalcmd(&mut self, data: h0031::Cmd<H>, na: bool) -> Result<(), CommandError> {
        // Create appropriately sized buffer
        let mut buf = HidIoPacketBuffer {
            // Test packet id
            id: HidIoCommandId::TerminalCmd,
            // Detect max size
            max_len: self.default_packet_chunk(),
            // Use defaults for other fields
            ..Default::default()
        };

        // Set NA (no-ack)
        if na {
            buf.ptype = HidIoPacketType::NaData;
        }

        // Build payload
        if !buf.append_payload(data.command.as_bytes()) {
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
            HidIoCommandId::TerminalCmd,
            HidIoPacketType::NaData,
        ))
    }
    fn h0031_terminalcmd_ack(&mut self, _data: h0031::Ack) -> Result<(), CommandError> {
        Err(CommandError::IdNotImplemented(
            HidIoCommandId::TerminalCmd,
            HidIoPacketType::Ack,
        ))
    }
    fn h0031_terminalcmd_nak(&mut self, _data: h0031::Nak) -> Result<(), CommandError> {
        Err(CommandError::IdNotImplemented(
            HidIoCommandId::TerminalCmd,
            HidIoPacketType::Nak,
        ))
    }
    fn h0031_terminalcmd_handler(&mut self, buf: HidIoPacketBuffer<H>) -> Result<(), CommandError> {
        // Handle packet type
        match buf.ptype {
            HidIoPacketType::Data => {
                // Copy data into struct
                let mut cmd = h0031::Cmd::<H> {
                    command: String::new(),
                };
                cmd.command
                    .push_str(match core::str::from_utf8(&buf.data) {
                        Ok(string) => string,
                        Err(e) => {
                            return Err(CommandError::InvalidUtf8(Utf8Error::new(e)));
                        }
                    })
                    .unwrap();

                match self.h0031_terminalcmd_cmd(cmd) {
                    Ok(_ack) => self.empty_ack(buf.id),
                    Err(_nak) => self.empty_nak(buf.id),
                }
            }
            HidIoPacketType::NaData => {
                // Copy data into struct
                let mut cmd = h0031::Cmd::<H> {
                    command: String::new(),
                };
                cmd.command
                    .push_str(match core::str::from_utf8(&buf.data) {
                        Ok(string) => string,
                        Err(e) => {
                            return Err(CommandError::InvalidUtf8(Utf8Error::new(e)));
                        }
                    })
                    .unwrap();

                self.h0031_terminalcmd_nacmd(cmd)
            }
            HidIoPacketType::Ack => self.h0031_terminalcmd_ack(h0031::Ack {}),
            HidIoPacketType::Nak => self.h0031_terminalcmd_nak(h0031::Nak {}),
            _ => Ok(()),
        }
    }

    fn h0034_terminalout(&mut self, data: h0034::Cmd<H>, na: bool) -> Result<(), CommandError> {
        // Create appropriately sized buffer
        let mut buf = HidIoPacketBuffer {
            // Test packet id
            id: HidIoCommandId::TerminalOut,
            // Detect max size
            max_len: self.default_packet_chunk(),
            // Use defaults for other fields
            ..Default::default()
        };

        // Set NA (no-ack)
        if na {
            buf.ptype = HidIoPacketType::NaData;
        }

        // Build payload
        if !buf.append_payload(data.output.as_bytes()) {
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
            HidIoCommandId::TerminalOut,
            HidIoPacketType::NaData,
        ))
    }
    fn h0034_terminalout_ack(&mut self, _data: h0034::Ack) -> Result<(), CommandError> {
        Err(CommandError::IdNotImplemented(
            HidIoCommandId::TerminalOut,
            HidIoPacketType::Ack,
        ))
    }
    fn h0034_terminalout_nak(&mut self, _data: h0034::Nak) -> Result<(), CommandError> {
        Err(CommandError::IdNotImplemented(
            HidIoCommandId::TerminalOut,
            HidIoPacketType::Nak,
        ))
    }
    fn h0034_terminalout_handler(&mut self, buf: HidIoPacketBuffer<H>) -> Result<(), CommandError> {
        // Handle packet type
        match buf.ptype {
            HidIoPacketType::Data => {
                // Copy data into struct
                let mut cmd = h0034::Cmd::<H> {
                    output: String::new(),
                };
                cmd.output
                    .push_str(match core::str::from_utf8(&buf.data) {
                        Ok(string) => string,
                        Err(e) => {
                            return Err(CommandError::InvalidUtf8(Utf8Error::new(e)));
                        }
                    })
                    .unwrap();

                match self.h0034_terminalout_cmd(cmd) {
                    Ok(_ack) => self.empty_ack(buf.id),
                    Err(_nak) => self.empty_nak(buf.id),
                }
            }
            HidIoPacketType::NaData => {
                // Copy data into struct
                let mut cmd = h0034::Cmd::<H> {
                    output: String::new(),
                };
                cmd.output
                    .push_str(match core::str::from_utf8(&buf.data) {
                        Ok(string) => string,
                        Err(e) => {
                            return Err(CommandError::InvalidUtf8(Utf8Error::new(e)));
                        }
                    })
                    .unwrap();

                self.h0034_terminalout_nacmd(cmd)
            }
            HidIoPacketType::Ack => self.h0034_terminalout_ack(h0034::Ack {}),
            HidIoPacketType::Nak => self.h0034_terminalout_nak(h0034::Nak {}),
            _ => Ok(()),
        }
    }

    fn h0050_manufacturing(&mut self, data: h0050::Cmd) -> Result<(), CommandError> {
        // Create appropriately sized buffer
        let mut buf = HidIoPacketBuffer {
            // Test packet id
            id: HidIoCommandId::ManufacturingTest,
            // Detect max size
            max_len: self.default_packet_chunk(),
            // Use defaults for other fields
            ..Default::default()
        };

        // Build payload
        if !buf.append_payload(&(data.command as u16).to_le_bytes()) {
            return Err(CommandError::DataVecTooSmall);
        }
        if !buf.append_payload(unsafe { &data.argument.raw.to_le_bytes() }) {
            return Err(CommandError::DataVecTooSmall);
        }

        buf.done = true;

        self.tx_packetbuffer_send(&mut buf)
    }
    fn h0050_manufacturing_cmd(&mut self, _data: h0050::Cmd) -> Result<h0050::Ack, h0050::Nak> {
        Err(h0050::Nak {})
    }
    fn h0050_manufacturing_ack(&mut self, _data: h0050::Ack) -> Result<(), CommandError> {
        Err(CommandError::IdNotImplemented(
            HidIoCommandId::ManufacturingTest,
            HidIoPacketType::Ack,
        ))
    }
    fn h0050_manufacturing_nak(&mut self, _data: h0050::Nak) -> Result<(), CommandError> {
        Err(CommandError::IdNotImplemented(
            HidIoCommandId::ManufacturingTest,
            HidIoPacketType::Nak,
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
                let command = h0050::Command::try_from(u16::from_le_bytes(
                    buf.data[0..2].try_into().unwrap(),
                ))
                .unwrap();
                let argument = h0050::Argument {
                    raw: u16::from_le_bytes(buf.data[2..4].try_into().unwrap()),
                };

                match self.h0050_manufacturing_cmd(h0050::Cmd { command, argument }) {
                    Ok(_ack) => self.empty_ack(buf.id),
                    Err(_nak) => self.empty_nak(buf.id),
                }
            }
            HidIoPacketType::NaData => Err(CommandError::InvalidPacketBufferType(buf.ptype)),
            HidIoPacketType::Ack => self.h0050_manufacturing_ack(h0050::Ack {}),
            HidIoPacketType::Nak => self.h0050_manufacturing_nak(h0050::Nak {}),
            _ => Ok(()),
        }
    }

    fn h0051_manufacturingres(&mut self, data: h0051::Cmd<HSUB4>) -> Result<(), CommandError> {
        // Create appropriately sized buffer
        let mut buf = HidIoPacketBuffer {
            // Test packet id
            id: HidIoCommandId::ManufacturingResult,
            // Detect max size
            max_len: self.default_packet_chunk(),
            // Use defaults for other fields
            ..Default::default()
        };

        // Build payload
        if !buf.append_payload(&(data.command as u16).to_le_bytes()) {
            return Err(CommandError::DataVecTooSmall);
        }
        if !buf.append_payload(unsafe { &data.argument.raw.to_le_bytes() }) {
            return Err(CommandError::DataVecTooSmall);
        }
        if !buf.append_payload(&data.data) {
            return Err(CommandError::DataVecTooSmall);
        }

        buf.done = true;
        trace!("h0051_manufacturingres: {:?} - {:?}", data, buf);

        self.tx_packetbuffer_send(&mut buf)
    }
    fn h0051_manufacturingres_cmd(
        &mut self,
        _data: h0051::Cmd<HSUB4>,
    ) -> Result<h0051::Ack, h0051::Nak> {
        Err(h0051::Nak {})
    }
    fn h0051_manufacturingres_ack(&mut self, _data: h0051::Ack) -> Result<(), CommandError> {
        Err(CommandError::IdNotImplemented(
            HidIoCommandId::ManufacturingResult,
            HidIoPacketType::Ack,
        ))
    }
    fn h0051_manufacturingres_nak(&mut self, _data: h0051::Nak) -> Result<(), CommandError> {
        Err(CommandError::IdNotImplemented(
            HidIoCommandId::ManufacturingResult,
            HidIoPacketType::Nak,
        ))
    }
    fn h0051_manufacturingres_handler(
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
                let command = h0051::Command::try_from(u16::from_le_bytes(
                    buf.data[0..2].try_into().unwrap(),
                ))
                .unwrap();
                let argument = h0051::Argument {
                    raw: u16::from_le_bytes(buf.data[2..4].try_into().unwrap()),
                };
                let data: Vec<u8, HSUB4> = if buf.data.len() > 4 {
                    Vec::from_slice(&buf.data[4..]).unwrap()
                } else {
                    Vec::new()
                };

                match self.h0051_manufacturingres_cmd(h0051::Cmd {
                    command,
                    argument,
                    data,
                }) {
                    Ok(_ack) => self.empty_ack(buf.id),
                    Err(_nak) => self.empty_nak(buf.id),
                }
            }
            HidIoPacketType::NaData => Err(CommandError::InvalidPacketBufferType(buf.ptype)),
            HidIoPacketType::Ack => self.h0051_manufacturingres_ack(h0051::Ack {}),
            HidIoPacketType::Nak => self.h0051_manufacturingres_nak(h0051::Nak {}),
            _ => Ok(()),
        }
    }
}
