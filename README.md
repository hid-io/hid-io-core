# hid-io
HID-IO Client Side Library and Daemon

[![Travis Status](https://travis-ci.org/hid-io/hid-io.svg?branch=master)](https://travis-ci.org/hid-io/hid-io) [![Appveyor Status](https://ci.appveyor.com/api/projects/status/cdwt6apvvfn4fvt9/branch/master?svg=true)](https://ci.appveyor.com/project/kiibohd/hid-io/branch/master)

[![Visit our IRC channel](https://kiwiirc.com/buttons/irc.freenode.net/hid-io.png)](https://kiwiirc.com/client/irc.freenode.net/#hid-io)

### API Documentation

* [master](https://hid-io.github.io/hid_io)


## Getting

Currently you have to build the HID-IO daemon yourself. But it will be made available in binary form once we are ready for a beta.


## Dependencies

* Rust >= 1.1.17 (may relax this over time)
* capnproto >= 0.6.0


## Building

```bash
cargo build
```

## Testing

```bash
RUST_LOG=info RUST_BACKTRACE=1 cargo run
```

## Supported Keyboard Firmware

* [kiibohd](https://github.com/kiibohd/controller) (KLL) - **In Progress**

