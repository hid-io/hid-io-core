/* Copyright (C) 2020 by Jacob Alexander
 *
 * This file is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This file is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this file.  If not, see <http://www.gnu.org/licenses/>.
 */

use crate::api::common_capnp;
/// HID-IO Core Daemon Node
/// Handles API queries directly to HID-IO Core rather than to a specific device
/// This is the standard way to interact with HID-IO Core modules from the capnp API
///
/// For the most part, this is a dummy node used mainly for node accounting with the mailbox.
/// The capnproto API should call the internal functions directly if possible.
use crate::api::Endpoint;
use crate::mailbox;
use std::sync::Arc;

pub struct DaemonNode {
    mailbox: mailbox::Mailbox,
    uid: u64,
    _endpoint: Endpoint,
}

impl DaemonNode {
    pub fn new(mailbox: mailbox::Mailbox) -> std::io::Result<DaemonNode> {
        // Assign a uid
        let uid = match mailbox.clone().assign_uid("".to_string(), "".to_string()) {
            Ok(uid) => uid,
            Err(e) => {
                panic!("Only 1 daemon node may be allocated: {}", e);
            }
        };

        // Setup Endpoint
        let mut endpoint = Endpoint::new(common_capnp::NodeType::HidioDaemon, uid);
        endpoint.set_daemonnode_params();

        // Register node
        mailbox.clone().register_node(endpoint.clone());

        Ok(DaemonNode {
            mailbox,
            uid,
            _endpoint: endpoint,
        })
    }
}

impl Drop for DaemonNode {
    fn drop(&mut self) {
        warn!("DaemonNode deallocated");

        // Unregister node
        self.mailbox.unregister_node(self.uid);
    }
}

pub async fn initialize(_rt: Arc<tokio::runtime::Runtime>, mailbox: mailbox::Mailbox) {
    // Event loop for Daemon Node (typically not used)
    tokio::spawn(async {
        let node = DaemonNode::new(mailbox).unwrap();
        info!("Initializing daemon node... uid:{}", node.uid);
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    });
}
