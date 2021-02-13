#![cfg(feature = "displayserver")]
/* Copyright (C) 2019-2021 by Jacob Alexander
 * Copyright (C) 2019 by Rowan Decker
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

#[cfg(target_os = "linux")]
/// Xorg impementation
pub mod x11;

#[cfg(target_os = "linux")]
/// Wayland impementation
pub mod wayland;

#[cfg(target_os = "windows")]
/// Winapi impementation
pub mod winapi;

#[cfg(target_os = "macos")]
/// macOS quartz impementation
pub mod quartz;

use crate::mailbox;
use crate::RUNNING;
use hid_io_protocol::{HidIoCommandId, HidIoPacketType};
use std::string::FromUtf8Error;
use std::sync::atomic::Ordering;
use tokio::stream::StreamExt;

#[cfg(all(feature = "displayserver", target_os = "linux"))]
use crate::module::displayserver::x11::*;

#[cfg(all(feature = "displayserver", target_os = "linux"))]
use crate::module::displayserver::wayland::*;

#[cfg(all(feature = "displayserver", target_os = "windows"))]
use crate::module::displayserver::winapi::*;

#[cfg(all(feature = "displayserver", target_os = "macos"))]
use crate::module::displayserver::quartz::*;

/// Functions that can be called in a cross platform manner
pub trait DisplayOutput {
    fn get_layout(&self) -> Result<String, DisplayOutputError>;
    fn set_layout(&self, layout: &str) -> Result<(), DisplayOutputError>;
    fn type_string(&mut self, string: &str) -> Result<(), DisplayOutputError>;
    fn press_symbol(&mut self, c: char, state: bool) -> Result<(), DisplayOutputError>;
    fn get_held(&mut self) -> Result<Vec<char>, DisplayOutputError>;
    fn set_held(&mut self, string: &str) -> Result<(), DisplayOutputError>;
}

#[derive(Debug)]
pub enum DisplayOutputError {
    AllocationFailed(char),
    Connection(String),
    Format(std::io::Error),
    General(String),
    LostConnection,
    NoKeycode,
    SetLayoutFailed(String),
    Unimplemented,
    Utf(FromUtf8Error),
}

impl std::fmt::Display for DisplayOutputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DisplayOutputError::AllocationFailed(e) => write!(f, "Allocation failed: {}", e),
            DisplayOutputError::Connection(e) => write!(f, "Connection: {}", e),
            DisplayOutputError::Format(e) => write!(f, "Format: {}", e),
            DisplayOutputError::General(e) => write!(f, "General: {}", e),
            DisplayOutputError::LostConnection => write!(f, "Lost connection"),
            DisplayOutputError::NoKeycode => write!(f, "No keycode mapped"),
            DisplayOutputError::SetLayoutFailed(e) => write!(f, "set_layout() failed: {}", e),
            DisplayOutputError::Unimplemented => write!(f, "Unimplemented"),
            DisplayOutputError::Utf(e) => write!(f, "UTF: {}", e),
        }
    }
}

impl From<std::io::Error> for DisplayOutputError {
    fn from(e: std::io::Error) -> Self {
        DisplayOutputError::Format(e)
    }
}

#[derive(Default)]
/// Dummy impementation for unsupported platforms
pub struct StubOutput {}

impl StubOutput {
    pub fn new() -> StubOutput {
        StubOutput {}
    }
}

impl DisplayOutput for StubOutput {
    fn get_layout(&self) -> Result<String, DisplayOutputError> {
        warn!("Unimplemented");
        Err(DisplayOutputError::Unimplemented)
    }
    fn set_layout(&self, _layout: &str) -> Result<(), DisplayOutputError> {
        warn!("Unimplemented");
        Err(DisplayOutputError::Unimplemented)
    }
    fn type_string(&mut self, _string: &str) -> Result<(), DisplayOutputError> {
        warn!("Unimplemented");
        Err(DisplayOutputError::Unimplemented)
    }
    fn press_symbol(&mut self, _c: char, _state: bool) -> Result<(), DisplayOutputError> {
        warn!("Unimplemented");
        Err(DisplayOutputError::Unimplemented)
    }
    fn get_held(&mut self) -> Result<Vec<char>, DisplayOutputError> {
        warn!("Unimplemented");
        Err(DisplayOutputError::Unimplemented)
    }
    fn set_held(&mut self, _string: &str) -> Result<(), DisplayOutputError> {
        warn!("Unimplemented");
        Err(DisplayOutputError::Unimplemented)
    }
}

/// Our "internal" node responsible for handling required commands
struct Module {
    display: Box<dyn DisplayOutput>,
}

#[cfg(not(feature = "displayserver"))]
fn get_display() -> Box<dyn DisplayOutput> {
    Box::new(StubOutput::new())
}

#[cfg(all(feature = "displayserver", target_os = "linux"))]
fn get_display() -> Box<dyn DisplayOutput> {
    // First attempt to connect to Wayland
    let wayland = WaylandConnection::new();
    match wayland {
        Ok(wayland) => Box::new(wayland),
        Err(_) => Box::new(XConnection::new()),
    }
}

#[cfg(all(feature = "displayserver", target_os = "windows"))]
fn get_display() -> Box<dyn DisplayOutput> {
    Box::new(DisplayConnection::new())
}

#[cfg(all(feature = "displayserver", target_os = "macos"))]
fn get_display() -> Box<dyn DisplayOutput> {
    Box::new(QuartzConnection::new())
}

impl Module {
    fn new() -> Module {
        let connection = get_display();

        match connection.get_layout() {
            Ok(layout) => {
                info!("Current layout: {}", layout);
            }
            Err(_) => {
                warn!("Failed to retrieve layout");
            }
        }

        Module {
            display: connection,
        }
    }
}

/// Supported Ids by this module
pub fn supported_ids() -> Vec<HidIoCommandId> {
    vec![
        HidIoCommandId::UnicodeText,
        HidIoCommandId::UnicodeState,
        HidIoCommandId::GetInputLayout,
        HidIoCommandId::SetInputLayout,
    ]
}

async fn process(mailbox: mailbox::Mailbox) {
    // Top-level module setup
    let mut module = Module::new();

    // Setup receiver stream
    let sender = mailbox.clone().sender.clone();
    let receiver = sender.clone().subscribe();
    tokio::pin! {
        let stream = receiver.into_stream()
            .filter(Result::is_ok).map(Result::unwrap)
            .filter(|msg| msg.dst == mailbox::Address::Module)
            .filter(|msg| supported_ids().contains(&msg.data.id))
            .filter(|msg| msg.data.ptype == HidIoPacketType::Data || msg.data.ptype == HidIoPacketType::NaData);
    }

    // Process filtered message stream
    while let Some(msg) = stream.next().await {
        let mydata = msg.data.data.clone();
        debug!("Processing command: {:?}", msg.data.id);
        match msg.data.id {
            HidIoCommandId::UnicodeText => {
                let s = String::from_utf8(mydata.to_vec()).unwrap();
                debug!("UnicodeText (start): {}", s);
                match module.display.type_string(&s) {
                    Ok(_) => {
                        msg.send_ack(sender.clone(), vec![]);
                    }
                    Err(_) => {
                        warn!("Failed to type Unicode string");
                        msg.send_nak(sender.clone(), vec![]);
                    }
                }
                debug!("UnicodeText (done): {}", s);
            }
            HidIoCommandId::UnicodeState => {
                let s = String::from_utf8(mydata.to_vec()).unwrap();
                debug!("UnicodeState (start): {}", s);
                match module.display.set_held(&s) {
                    Ok(_) => {
                        msg.send_ack(sender.clone(), vec![]);
                    }
                    Err(_) => {
                        warn!("Failed to set Unicode key");
                        msg.send_nak(sender.clone(), vec![]);
                    }
                }
                debug!("UnicodeState (done): {}", s);
            }
            HidIoCommandId::GetInputLayout => {
                debug!("GetInputLayout (start)");
                match module.display.get_layout() {
                    Ok(layout) => {
                        info!("Current layout: {}", layout);
                        msg.send_ack(sender.clone(), layout.as_bytes().to_vec());
                    }
                    Err(_) => {
                        warn!("Failed to get input layout");
                        msg.send_nak(sender.clone(), vec![]);
                    }
                }
                debug!("GetInputLayout (done)");
            }
            HidIoCommandId::SetInputLayout => {
                let s = String::from_utf8(mydata.to_vec()).unwrap();
                debug!("SetInputLayout (start): {}", s);
                /* TODO - Setting layout is more complicated for X11 (and Wayland)
                info!("Setting language to {}", s);
                msg.send_ack(sender.clone(), vec![]);
                */
                warn!("Not implemented");
                msg.send_nak(sender.clone(), vec![]);
                debug!("SetInputLayout (done): {}", s);
            }
            _ => {}
        }
    }
}

/// Display Server initialization
/// The display server module selection the OS native display server to start.
/// Depending on the native display server not all of the functionality may be available.
pub async fn initialize(mailbox: mailbox::Mailbox) {
    // Setup local thread
    // This confusing block spawns a dedicated thread, and then runs a task LocalSet inside of it
    // This is required to avoid the use of the Send trait.
    // hid-io-core requires multiple threads like this which can dead-lock each other if run from
    // the same thread (which is the default behaviour of task LocalSet spawn_local)
    let rt = mailbox.rt.clone();
    rt.clone()
        .spawn_blocking(move || {
            rt.block_on(async {
                let local = tokio::task::LocalSet::new();
                local.spawn_local(process(mailbox));

                // Wait for exit signal before cleaning up
                local
                    .run_until(async move {
                        loop {
                            if !RUNNING.load(Ordering::SeqCst) {
                                break;
                            }
                            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                        }
                    })
                    .await;
            });
        })
        .await
        .unwrap();
}
