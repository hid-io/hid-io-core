/* Copyright (C) 2017-2020 by Jacob Alexander
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

use crate::mailbox;
use crate::RUNNING;
use std::sync::atomic::Ordering;

const SLEEP_DURATION: u64 = 100;

/// debug processing
async fn processing() {
    info!("Spawning device/debug spawning thread...");

    // Loop infinitely, the watcher only exits if the daemon is quit
    loop {
        if !RUNNING.load(Ordering::SeqCst) {
            break;
        }

        // Sleep so we don't starve the CPU
        tokio::time::delay_for(std::time::Duration::from_millis(SLEEP_DURATION)).await;
    }
}

/// device debug module initialization
///
/// # Arguments
///
/// # Remarks
///
/// Sets up a processing thread for the debug module.
///
pub async fn initialize(_mailbox: mailbox::Mailbox) {
    info!("Initializing device/debug...");

    // Spawn watcher thread
    tokio::spawn(processing()).await.unwrap()
}
