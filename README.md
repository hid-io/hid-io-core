# hid-io
HID-IO Client Side Library and Daemon

[![Travis Status](https://travis-ci.org/hid-io/hid-io.svg?branch=master)](https://travis-ci.org/hid-io/hid-io) [![Appveyor Status](https://ci.appveyor.com/api/projects/status/cdwt6apvvfn4fvt9/branch/master?svg=true)](https://ci.appveyor.com/project/kiibohd/hid-io/branch/master)

[![Visit our IRC channel](https://kiwiirc.com/buttons/irc.freenode.net/hid-io.png)](https://kiwiirc.com/client/irc.freenode.net/#hid-io)

### API Documentation

* [master](https://hid-io.github.io/hid_io)


## Getting

Currently you have to build the HID-IO daemon yourself. But it will be made available in binary form once we are ready for a beta.


## Usage

```bash
hid-io
hid-io --help
```


## Dependencies

* Rust >= 1.17.0 (may relax (or tighten) this over time)
* capnproto >= 0.6.0


### i686-pc-windows-gnu Dependencies

* `make` must be path


## Building

```bash
cargo build
```


## Testing

```bash
RUST_LOG=info RUST_BACKTRACE=1 cargo run
```


### Running Unit Tests

```bash
cargo test
```

## Supported Keyboard Firmware

* [kiibohd](https://github.com/kiibohd/controller) (KLL) - **In Progress**




`sudo usbhid-dump -m 308f:0013 -es`
`sudo usbhid-dump -m 1c11:b04d -es -t 0 -i 5`


# Debugging

`echo 0 | sudo tee /proc/sys/kernel/yama/ptrace_scope`
`rust-gdb target/debug/hid-io -p $(pidof hid-io)`

# Packaging
`cargo build --release --target "x86_64-pc-windows-gnu"`

#

`install_service.exe`
`sc start hid-io`
`sc stop hid-io`
`sc query hid-io`

#

`cp hidio.plist ~/Library/LaunchAgents`
`launchctl -w  ~/Library/LaunchAgents/hidio.plist`
