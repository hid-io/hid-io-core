# hid-io protocol

[![docs.rs](https://docs.rs/hid-io-protocol/badge.svg)](https://docs.rs/hid-io-protocol)
[![Crates.io](https://img.shields.io/crates/v/hid-io-protocol.svg)](https://crates.io/crates/hid-io-protocol)
[![Crates.io](https://img.shields.io/crates/l/hid-io-protocol.svg)](https://crates.io/crates/hid-io-protocol)
[![Crates.io](https://img.shields.io/crates/d/hid-io-protocol.svg)](https://crates.io/crates/hid-io-protocol)

HID-IO Server and Device protocol implementation

This library can be integrated into both embedded and full user-space applications using the `device` and `server` feature flags.

The hid-io protocol library handles these features:

- Buffer packetization and re-assembly
- All the different packet types (Data, ACK, NAK, No ACK Data, Sync as well as the continued variations)
- HID-IO Command processing (both send and receive)
- 16 and 32-bit command IDs


### Spec

[HID-IO Protocol Spec](spec)


### API Documentation

See [docs.rs](https://docs.rs/hid-io-protocol).


## Building

### Server Library

```bash
cargo build
cargo build --release
```


### Device Library

```bash
cargo build --no-default-features --features device
cargo build --no-default-features --features device --release
```


## Usage

There are two different ways to utilize hid-io-protocol as a server library.

1. Ingest bytes, assemble buffer, handle message, create response buffer, serialize buffer, send bytes
2. Ingest message buffer, handle message, create response buffer, send buffer

In each way a CommandInterface struct is created and the Commands trait is implemented. The implementation of CommandInterface is what differentiates betwen the two options.

```rust

```

Option 1 is simpler as hid-io-protocol can handle all processing from a hidraw interface. Device libraries usually go for this.

```rust
const BufChunk: usize = U64;
const IdLen: usize = U10;
const MessageLen: usize = U256;
const RxBuf: usize = U8;
const SerializationLen: usize = U276;
const TxBuf: usize = U8;

let ids = [
    HidIoCommandID::SupportedIDs,
    /* Add supported ids */
    /* This is the master list, if it's not listed here comamnds will not work */
];

let intf = CommandInterface::<TxBuf, RxBuf, BufChunk, MessageLen, {MessageLen - 1}, {MessageLen - 4}, SerializationLen, IdLen>::new(&ids).unwrap();
}

// The max length must equal BufChunk (e.g. 64 bytes)
// This may not be 64 bytes depending on your use-case and situation
// 63 bytes is common when you need to use a hid report id
let hidraw_buffer = read_hidraw();

// Enqueue bytes into buffer
intf.rx_bytebuf.enqueue(match Vec::from_slice(slice) {
    Ok(vec) => vec,
    Err(_) => {
        return HidioStatus::ErrorBufSizeTooSmall;
    }
}).unwrap();

// Process messages
// If any responses are created, they'll be sent out with intf.tx_bytebuf
intf.process_rx();

// Copy a single chunk from the tx_buffer
// You'll likely want to do this repeatedly until the buffer is empty
match intf.tx_bytebuf.dequeue() {
    Some(chunk) => {
        // Write to hidraw output buffer
        // Same size restrictions apply as above
        write_hidraw(chunk);
    }
    None => {}
}

struct CommandInterface<
    const TX: usize,
    const RX: usize,
    const N: usize,
    const H: usize,
    const HSUB1: usize,
    const HSUB4: usize,
    const S: usize,
    const ID: usize,
> {
    ids: Vec<HidIoCommandID, ID>,
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
        const HSUB4: usize,
        const S: usize,
        const ID: usize,
    > CommandInterface<TX, RX, N, H, HSUB1, HSUB4, S, ID>
{
    fn new(
        ids: &[HidIoCommandID],
    ) -> Result<CommandInterface<TX, RX, N, H, HSUB1, HSUB4, S, ID>, CommandError> {
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
                                    // Clear buffer, packet missing
                                    self.rx_packetbuf.clear();
                                }
                                _ => {
                                    return Ok(true);
                                }
                            }
                        }
                    }
                    Err(e) => {
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
    /// Returns the number of buffers processed
    pub fn process_rx(&mut self) -> Result<u8, CommandError> {
        // Decode bytes into buffer
        while (self.rx_packetbuffer_decode()? {
            // Process rx buffer
            self.rx_message_handling(self.rx_packetbuf.clone())?;

            // Clear buffer
            self.rx_packetbuf.clear();
        }

        Ok(cur)
    }
}

/// CommandInterface for Commands
/// TX - tx byte buffer size (in multiples of N)
/// RX - tx byte buffer size (in multiples of N)
/// N - Max payload length (HidIoPacketBuffer), used for default values
/// H - Max data payload length (HidIoPacketBuffer)
/// S - Serialization buffer size
/// ID - Max number of HidIoCommandIDs
impl<
        const TX: usize,
        const RX: usize,
        const N: usize,
        const H: usize,
        const HSUB1: usize,
        const HSUB4: usize,
        const S: usize,
        const ID: usize,
    > Commands<H, ID> for CommandInterface<TX, RX, N, H, HSUB1, HSUB4, S, ID> {
    fn default_packet_chunk(&self) -> u32 {
        N as u32
    }

    fn tx_packetbuffer_send(&mut self, buf: &mut HidIoPacketBuffer<H>) -> Result<(), CommandError> {
        let size = buf.serialized_len() as usize;
        if self.serial_buf.resize_default(size).is_err() {
            return Err(CommandError::SerializationVecTooSmall);
        }
        match buf.serialize_buffer(&mut self.serial_buf) {
            Ok(data) => data,
            Err(err) => {
                return Err(CommandError::SerializationFailed(err));
            }
        };

        // Add serialized data to buffer
        // May need to enqueue multiple packets depending how much
        // was serialized
        // The first byte is a serde type and is dropped
        let data = &self.serial_buf;
        for pos in (1..data.len()).step_by(N) {
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
    fn supported_id(&self, id: HidIoCommandID) -> bool {
        /* Your implementation */
    }

    fn h0000_supported_ids_cmd(&mut self, _data: h0000::Cmd) -> Result<h0000::Ack<ID>, h0000::Nak> {
        /* Message specific commands are optional to implement */
    }
}
```

hid-io-core uses Option 2 as different threads handle byte ingest and message handling. You'll need to handle buffer assembly/disassembly yourself.

```rust
        struct CommandInterface {}
        impl Commands<mailbox::HidIoPacketBufferDataSize, 0> for CommandInterface {
            fn tx_packetbuffer_send(
                &mut self,
                buf: &mut mailbox::HidIoPacketBuffer,
            ) -> Result<(), CommandError> {
                /* Send command and wait for a reply */
                /* If sending an ACK/NAK or NA Data then there won't be a reply */

                if let Some(rcvmsg) = /* send buffer */ {
                    // Handle ack/nak
                    self.rx_message_handling(rcvmsg.data)?;
                }
                Ok(())
            }
            fn h0016_flashmode_ack(
                &mut self,
                data: h0016::Ack,
            ) -> Result<(), CommandError> {
                /* ACK */
                Ok(())
            }
            fn h0016_flashmode_nak(
                &mut self,
                data: h0016::Nak,
            ) -> Result<(), CommandError> {
                /* NAK */
                Ok(())
            }
        }
        let mut intf = CommandInterface {};

        // Send command
        intf.h0016_flashmode(h0016::Cmd {}).unwrap();
```

### hidraw Setup

When using hid-io-protocol with device firmware you'll need to setup a hidraw interface. This should work the same for both USB, Bluetooth (and anything else that supports the USB HID spec).
Option 1 in the usage examples has stubs indicating where hidraw rx and tx occur.

#### HID Descriptor

Below is an example hidraw HID descriptor for USB 2.0 FS using 64 byte packets.
```c
    0x06, 0x1C, 0xFF,    // Usage Page (Vendor Defined) 0xFF1C
    0x0A, 0x00, 0x11,    // Usage 0x1100
    0xA1, 0x01,          // Collection (Application)
    0x75, 0x08,          //   Report Size (8)
    0x15, 0x00,          //   Logical Minimum (0)
    0x26, 0xFF, 0x00,    //   Logical Maximum (255)

    0x95, 0x40,          //     Report Count (64)
    0x09, 0x01,          //     Usage (Output)
    0x91, 0x02,          //     Output (Data,Var,Abs)

    0x95, 0x40,          //     Report Count (64)
    0x09, 0x02,          //     Usage (Input)
    0x81, 0x02,          //     Input (Data,Var,Abs)

    0xC0,                // End Collection
```

It's also possible to use Report Ids; however the report count should be adjusted to make sure you're not sending multiple USB packets for a single hidraw report. Adjust the **Report Count** fields accordingly.

#### USB Endpoint Setup

Below is an example of how to setup the USB endpoints for the above hid descriptor.

```c
// --- Vendor Specific / RAW I/O ---
// - 9 bytes -
	// interface descriptor, USB spec 9.6.5, page 267-269, Table 9-12
	9,                                      // bLength
	4,                                      // bDescriptorType
	RAWIO_INTERFACE,                        // bInterfaceNumber
	0,                                      // bAlternateSetting
	2,                                      // bNumEndpoints
	0x03,                                   // bInterfaceClass (0x03)
	0x00,                                   // bInterfaceSubClass
	0x00,                                   // bInterfaceProtocol
	0,                                      // iInterface (can point to a string name if desired)

// - 9 bytes -
	// HID interface descriptor, HID 1.11 spec, section 6.2.1
	9,                                      // bLength
	0x21,                                   // bDescriptorType
	0x11, 0x01,                             // bcdHID
	0,                                      // bCountryCode
	1,                                      // bNumDescriptors
	0x22,                                   // bDescriptorType
	LSB(sizeof(rawio_report_desc)),         // wDescriptorLength
	MSB(sizeof(rawio_report_desc)),

// - 7 bytes -
	// endpoint descriptor, USB spec 9.6.6, page 269-271, Table 9-13
	7,                                      // bLength
	5,                                      // bDescriptorType
	RAWIO_TX_ENDPOINT | 0x80,               // bEndpointAddress
	0x03,                                   // bmAttributes (0x03=intr)
	0x40, 0,                                // wMaxPacketSize (64 bytes)
	1,                                      // bInterval

// - 7 bytes -
	// endpoint descriptor, USB spec 9.6.6, page 269-271, Table 9-13
	7,                                      // bLength
	5,                                      // bDescriptorType
	RAWIO_RX_ENDPOINT,                      // bEndpointAddress
	0x03,                                   // bmAttributes (0x03=intr)
	0x40, 0,                                // wMaxPacketSize (64 bytes)
	1,                                      // bInterval
```

Then all you should have to do is send and receive data to your specified endpoints `RAWIO_TX_ENDPOINT` and `RAWIO_RX_ENDPOINT`.


### C Firmware Usage

See [hid-io-kiibohd](../hid-io-kiibohd)


## Testing

```bash
cargo test
```

Some of the tests utilize additional logging so you can also do:
```bash
RUST_LOG=info cargo test
```


## Dependencies

* Rust nightly (may relax over time)
* **NOTE**: [bincode-core](https://github.com/bincode-org/bincode-core) is so new it doesn't have a proper release yet and may break at any time.


## Supported Server Applications

* [hid-io-core](https://github.com/hid-io/hid-io-core)


## Supported Device Firmware

* [kiibohd](https://github.com/kiibohd/controller) (KLL) - **In Progress**


## Contributing

* Pull-requests run a variety of tests
* When adding new messages, make sure to add a unit test validation
* Some recommended tests:
  - `cargo test`
  - `cargo build`
  - `cargo build --no-default-features --features device`
