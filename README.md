# hid-io
HID-IO Client Side Library and Daemon

![Overview](misc/images/HID-IO_Overview.png)

[![Linux Status](https://github.com/hid-io/hid-io/workflows/Rust%20Linux/badge.svg)](https://github.com/hid-io/hid-io/actions)
[![macOS Status](https://github.com/hid-io/hid-io/workflows/Rust%20macOS/badge.svg)](https://github.com/hid-io/hid-io/actions)
[![Windows Status](https://github.com/hid-io/hid-io/workflows/Rust%20Windows/badge.svg)](https://github.com/hid-io/hid-io/actions)
[![Doc Status](https://github.com/hid-io/hid-io/workflows/Rust%20Doc%20Deploy/badge.svg)](https://github.com/hid-io/hid-io/actions)



[![Visit our IRC channel](https://kiwiirc.com/buttons/irc.freenode.net/hid-io.png)](https://kiwiirc.com/client/irc.freenode.net/#hid-io)

### API Documentation

* [master](https://hid-io.github.io/hid_io)


## Getting

Currently you have to build the HID-IO daemon yourself. But it will be made available in binary form once we are ready for a public beta.


## Usage

```bash
hid-io
hid-io --help
```

## RPC Terminal Example
`cargo run --example rpc`

## Dependencies

* Rust nightly (may relax over time)
* capnproto >= 0.7.0


### i686-pc-windows-gnu Dependencies

* `make` must be path


## Building

```bash
cargo build
```


## Testing

```bash
RUST_LOG=hid_io=info RUST_BACKTRACE=1 cargo run
```

Inspecting rawhid traffic:

`sudo usbhid-dump -m 308f:0013 -es`
`sudo usbhid-dump -m 1c11:b04d -es -t 0 -i 5`


### Running Unit Tests

```bash
cargo test
```

## Supported Keyboard Firmware

* [kiibohd](https://github.com/kiibohd/controller) (KLL) - **In Progress**


## Contributing

* Please run `cargo test` before submitting a pull-request

* Travis will fail any commits that do not pass all tests


## Debugging

`echo 0 | sudo tee /proc/sys/kernel/yama/ptrace_scope`
`rust-gdb target/debug/hid-io -p $(pidof hid-io)`

## Packaging
`cargo build --release --target "x86_64-pc-windows-gnu"`

## Linux systemd service
`cp hid-io.service /etc/systemd/system`
`systemctl daemon-reload`
`systemctl enable --now hid-io`

## Windows service

`install_service.exe`
`sc start hid-io`
`sc stop hid-io`
`sc query hid-io`

## OSX service

`cp hidio.plist ~/Library/LaunchAgents`
`launchctl -w  ~/Library/LaunchAgents/hidio.plist`
