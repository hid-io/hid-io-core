# hid-io protocol

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

TODO


## Server Library

```bash
cargo build
cargo build --release
```


### Usage

TODO


## Device Library

```bash
cargo build --target thumbv7em-none-eabi --no-default-features --features device
cargo build --target thumbv7em-none-eabi --no-default-features --features device --release
```


### Rust Firmware Usage

TODO


### C Firmware Usage

TODO


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


```bash
cargo build
```


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
  - `cargo build --target thumbv7em-none-eabi --no-default-features --features device`
