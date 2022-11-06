# hid-io-client

HID-IO Client Side application interface

The purpose of this crate is to provide a common set of functions that can be used to connect directly to hid-io-core.
Please see [hid-io-client-ffi](../hid-io-client-ffi) if you are looking for an FFI-compatible library interface.

## Connecting to hid-io-core

```rust
extern crate tokio;

use hid_io_core::common_capnp::NodeType;
use hid_io_core::logging::setup_logging_lite;
use rand::Rng;

#[tokio::main]
pub async fn main() -> Result<(), ::capnp::Error> {
    setup_logging_lite().ok();
    tokio::task::LocalSet::new().run_until(try_main()).await
}

async fn try_main() -> Result<(), ::capnp::Error> {
    // Prepare hid-io-core connection
    let mut hidio_conn = hid_io_client::HidioConnection::new().unwrap();
    let mut rng = rand::thread_rng();

    // Connect and authenticate with hid-io-core
    let (hidio_auth, _hidio_server) = hidio_conn
        .connect(
            hid_io_client::AuthType::Priviledged,
            NodeType::HidioApi,
            "lsnodes".to_string(),
            format!("{:x} - pid:{}", rng.gen::<u64>(), std::process::id()),
            true,
            std::time::Duration::from_millis(1000),
        )
        .await?;
    let hidio_auth = hidio_auth.expect("Could not authenticate to hid-io-core");

    let nodes_resp = {
        let request = hidio_auth.nodes_request();
        request.send().promise.await?
    };
    let nodes = nodes_resp.get()?.get_nodes()?;

    println!();
    for n in nodes {
        println!(" * {} - {}", n.get_id(), hid_io_client::format_node(n));
    }

    hidio_conn.disconnect().await?;
    Ok(())
}
```
