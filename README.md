# hid-io
HIDIO Client Side Library and Daemon

[![Travis Status](https://travis-ci.org/hid-io/hid-io.svg?branch=master)](https://travis-ci.org/hid-io/hid-io) [![Appveyor Status](https://ci.appveyor.com/api/projects/status/cdwt6apvvfn4fvt9/branch/master?svg=true)](https://ci.appveyor.com/project/kiibohd/hid-io/branch/master)

[![Visit our IRC channel](https://kiwiirc.com/buttons/irc.freenode.net/hid-io.png)](https://kiwiirc.com/client/irc.freenode.net/#hid-io)

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

