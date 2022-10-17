/* Copyright (C) 2019-2022 by Jacob Alexander
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

use capnp::capability::Promise;
use hid_io_core::common_capnp::NodeType;
use hid_io_core::hidio_capnp::hid_io;
use hid_io_core::logging::setup_logging_lite;
use hid_io_core::HidIoCommandId;
use rand::Rng;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::fmt::Write;

struct Node {
    type_: NodeType,
    _name: String,
    _serial: String,
}

struct NodesSubscriberImpl {
    nodes_lookup: HashMap<u64, Node>,
    start_time: std::time::Instant,
}

impl NodesSubscriberImpl {
    fn new() -> NodesSubscriberImpl {
        let nodes_lookup: HashMap<u64, Node> = HashMap::new();
        let start_time = std::time::Instant::now();

        NodesSubscriberImpl {
            nodes_lookup,
            start_time,
        }
    }

    fn format_packet(&mut self, packet: hid_io::packet::Reader<'_>) -> String {
        let mut datastr = "".to_string();
        for b in packet.get_data().unwrap().iter() {
            write!(datastr, "{:02x}", b).unwrap();
        }
        let datalen = packet.get_data().unwrap().len();
        let src = packet.get_src();
        let src_node_type = if src == 0 {
            "All".to_string()
        } else if let Some(n) = self.nodes_lookup.get(&src) {
            format!("{:?}", n.type_)
        } else {
            format!("{:?}", NodeType::Unknown)
        };

        let dst = packet.get_dst();
        let dst_node_type = if dst == 0 {
            "All".to_string()
        } else if let Some(n) = self.nodes_lookup.get(&dst) {
            format!("{:?}", n.type_)
        } else {
            format!("{:?}", NodeType::Unknown)
        };

        // TODO (HaaTa): decode packets to show fields
        if datalen == 0 {
            format!(
                "{} - {:?}: {}:{}->{}:{} ({:?}:{}) Len:{}",
                self.start_time.elapsed().as_millis(),
                packet.get_type().unwrap(),
                src,
                src_node_type,
                dst,
                dst_node_type,
                HidIoCommandId::try_from(packet.get_id()).unwrap_or(HidIoCommandId::Unused),
                packet.get_id(),
                datalen,
            )
        } else {
            format!(
                "{} - {:?}: {}:{}->{}:{} ({:?}:{}) Len:{}\n\t{}",
                self.start_time.elapsed().as_millis(),
                packet.get_type().unwrap(),
                src,
                src_node_type,
                dst,
                dst_node_type,
                HidIoCommandId::try_from(packet.get_id()).unwrap_or(HidIoCommandId::Unused),
                packet.get_id(),
                datalen,
                datastr,
            )
        }
    }
}

impl hid_io::nodes_subscriber::Server for NodesSubscriberImpl {
    fn nodes_update(
        &mut self,
        params: hid_io::nodes_subscriber::NodesUpdateParams,
        _results: hid_io::nodes_subscriber::NodesUpdateResults,
    ) -> Promise<(), capnp::Error> {
        // Re-create nodes_lookup on each update
        self.nodes_lookup = HashMap::new();

        println!("nodes_update: ");
        for n in capnp_rpc::pry!(capnp_rpc::pry!(params.get()).get_nodes()) {
            println!("{} - {}", n.get_id(), hid_io_client::format_node(n));
            self.nodes_lookup.insert(
                n.get_id(),
                Node {
                    type_: n.get_type().unwrap(),
                    _name: n.get_name().unwrap_or("").to_string(),
                    _serial: n.get_serial().unwrap_or("").to_string(),
                },
            );
        }
        Promise::ok(())
    }

    fn hidio_watcher(
        &mut self,
        params: hid_io::nodes_subscriber::HidioWatcherParams,
        _results: hid_io::nodes_subscriber::HidioWatcherResults,
    ) -> Promise<(), capnp::Error> {
        println!(
            "{}",
            self.format_packet(capnp_rpc::pry!(capnp_rpc::pry!(params.get()).get_packet()))
        );
        Promise::ok(())
    }
}

#[tokio::main]
pub async fn main() -> Result<(), ::capnp::Error> {
    setup_logging_lite().ok();
    tokio::task::LocalSet::new().run_until(try_main()).await
}

async fn try_main() -> Result<(), ::capnp::Error> {
    // Prepare hid-io-core connection
    let mut hidio_conn = hid_io_client::HidioConnection::new().unwrap();
    let mut rng = rand::thread_rng();

    loop {
        // Connect and authenticate with hid-io-core
        let (hidio_auth, hidio_server) = hidio_conn
            .connect(
                hid_io_client::AuthType::Priviledged,
                NodeType::HidioApi,
                "watchnodes".to_string(),
                format!("{:x} - pid:{}", rng.gen::<u64>(), std::process::id()),
                true,
                std::time::Duration::from_millis(1000),
            )
            .await?;
        let hidio_auth = hidio_auth.expect("Could not authenticate to hid-io-core");

        // Subscribe to nodeswatcher
        let nodes_subscription = capnp_rpc::new_client(NodesSubscriberImpl::new());
        let mut request = hidio_auth.subscribe_nodes_request();
        request.get().set_subscriber(nodes_subscription);
        let _callback = request.send().promise.await;

        loop {
            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;

            // Check if the server is still alive
            let request = hidio_server.alive_request();
            if let Err(e) = request.send().promise.await {
                println!("Dead: {}", e);
                // Break the subscription loop and attempt to reconnect
                break;
            }
        }
    }
}
