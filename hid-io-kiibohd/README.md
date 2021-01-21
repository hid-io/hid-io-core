# hid-io-kiibohd

C FFI library to used within embedded firmware.
Specifically for the kiibohd keyboard firmware (https://github.com/kiibohd/controller/)

## Building

```bash
cargo install cargo-c
cargo cbuild
```

Note: There are some issues currently with cargo-c and workspaces.
You'll find the generated files in the top-level target directory.
