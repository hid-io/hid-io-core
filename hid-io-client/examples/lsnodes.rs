/* Copyright (C) 2019-2023 by Jacob Alexander
 * Copyright (C) 2019 by Rowan Decker
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

extern crate tokio;

use hid_io_client::capnp;
use hid_io_client::common_capnp::NodeType;
use hid_io_client::setup_logging_lite;
use rand::Rng;

#[tokio::main]
pub async fn main() -> Result<(), capnp::Error> {
    setup_logging_lite().ok();
    tokio::task::LocalSet::new().run_until(try_main()).await
}

async fn try_main() -> Result<(), capnp::Error> {
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
