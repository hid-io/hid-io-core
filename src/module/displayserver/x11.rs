/* Copyright (C) 2019-2020 by Jacob Alexander
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

use crate::module::displayserver::{DisplayOutput, DisplayOutputError};
use std::collections::HashMap;
use std::convert::TryInto;
use std::ffi::CString;
use std::os::raw::{c_int, c_uchar, c_void};
use std::process::Command;
use std::ptr::null;
use x11::xlib::*;
use x11::xtest::*;

// XXX (HaaTa): Not sure why we need an additional 50 ms for the sequence to stick and not
// to get overridden by the next sequence.
const KEY_SEQUENCE_END_DELAY_MS: u64 = 50;

pub struct XConnection {
    display: *mut x11::xlib::_XDisplay,
    charmap: HashMap<char, u32>,
    held: Vec<char>,
    last_event_before_delays: std::time::Instant, // Last instance event, only updated when enough time has passed to decrement pending delays
    pending_delays: i64,                          // Number of 1ms delays pending
}

impl Default for XConnection {
    fn default() -> Self {
        Self::new()
    }
}

impl XConnection {
    pub fn new() -> XConnection {
        unsafe {
            let display = XOpenDisplay(null());
            let charmap = HashMap::new();
            let held = Vec::new();
            let last_event_before_delays = std::time::Instant::now();
            let pending_delays = 0;
            XConnection {
                display,
                charmap,
                held,
                last_event_before_delays,
                pending_delays,
            }
        }
    }

    pub fn find_keycode(&self, keysym: u64) -> (bool, Option<u32>) {
        let display = self.display;
        let mut keycode = None;
        let mut empty = false;

        unsafe {
            let mut keycode_low: c_int = 0;
            let mut keycode_high: c_int = 0;
            XDisplayKeycodes(display, &mut keycode_low, &mut keycode_high);

            let mut keysyms_per_keycode: c_int = 0;
            let keysyms = XGetKeyboardMapping(
                display,
                keycode_low as c_uchar,
                keycode_high - keycode_low,
                &mut keysyms_per_keycode,
            );

            for i in keycode_low..keycode_high {
                empty = true;
                for j in 0..keysyms_per_keycode {
                    let symindex = (i - keycode_low) * keysyms_per_keycode + j;
                    let s = *keysyms.offset(symindex as isize);

                    let c = XKeysymToString(s);
                    if c.as_ref().is_some() {
                        let v = std::ffi::CStr::from_ptr(c);
                        trace!("sym[{},{}] = {} ({:?})", i, j, s, v.to_str().unwrap_or(""));
                    } else {
                        trace!("sym[{},{}] = {}", i, j, s);
                    }

                    if s == keysym {
                        empty = false;
                        keycode = Some(i as u32);
                        break;
                    }
                    if s != 0 {
                        empty = false;
                        break;
                    }
                }

                if empty {
                    keycode = Some(i as u32);
                    break;
                }
            }

            XFree(keysyms as *mut c_void);
        }

        (empty, keycode)
    }

    pub fn lookup_sym(symbol: char) -> u64 {
        let hex: u32 = symbol.into();
        let s = format!("U{:x}", hex);
        let xs = CString::new(s).unwrap();
        unsafe { XStringToKeysym(xs.as_ptr()) }
    }

    pub fn bind_key(&self, keycode: u32, keysym: u64) {
        unsafe {
            // https://stackoverflow.com/a/44334103
            let mut keysyms = [keysym, keysym];
            XChangeKeyboardMapping(
                self.display,
                keycode as i32,
                keysyms.len() as i32,
                keysyms.as_mut_ptr(),
                1,
            );
            XSync(self.display, false as i32);
        }
    }

    pub fn unbind_key(&self, keycode: u32) {
        unsafe {
            let mut no_sym: u64 = 0;
            XChangeKeyboardMapping(self.display, keycode as i32, 1, &mut no_sym, 1);
        }
    }

    fn update_pending_delays(&mut self) {
        // Update pending delay (if enough time has pass, can be set to zero)
        let elapsed_ms: i64 = self
            .last_event_before_delays
            .elapsed()
            .as_millis()
            .try_into()
            .unwrap();
        if elapsed_ms > self.pending_delays {
            self.pending_delays = 0; // Safe to send the event immediately
        } else {
            self.pending_delays -= elapsed_ms as i64 - 1; // Add a 1ms delay
        }
    }

    /// Press/Release a specific keycode
    /// NOTE: This call does not update the pending delays and must be updated prior
    /// (unlike press_release_key())
    pub fn press_key(&mut self, keycode: u32, press: bool) {
        unsafe {
            XTestFakeKeyEvent(
                self.display,
                keycode,
                press as i32,
                self.pending_delays as u64,
            );
        }
    }

    /// Press then release a specific keycode
    /// Faster than individual calls to press_key as you don't need a delay between press and
    /// release of the same keycode.
    /// This function will automatically add delays as necessary using previously calculated
    /// delays.
    pub fn press_release_key(&mut self, keycode: u32) {
        self.update_pending_delays();

        unsafe {
            XTestFakeKeyEvent(
                self.display,
                keycode,
                true as i32,
                self.pending_delays as u64,
            );
            XTestFakeKeyEvent(
                self.display,
                keycode,
                false as i32,
                self.pending_delays as u64,
            );
        }
    }

    pub fn map_sym(&mut self, c: char) -> Option<u32> {
        // Special character lookup, otherwise normal lookup
        let keysym = match c {
            '\n' => x11::keysym::XK_Return as u64,
            '\t' => x11::keysym::XK_Tab as u64,
            _ => XConnection::lookup_sym(c),
        };
        let (unmapped, keycode) = self.find_keycode(keysym);
        if let Some(keycode) = keycode {
            if unmapped {
                self.bind_key(keycode, keysym);
                self.charmap.insert(c, keycode);
            }
            Some(keycode)
        } else {
            warn!("Could not find available keycode");
            None
        }
    }

    pub fn unmap_sym(&mut self, c: char) {
        if let Some(keycode) = self.charmap.get(&c) {
            self.unbind_key(*keycode);
            self.charmap.remove(&c);
        }
    }

    pub fn get_sym(&mut self, c: char) -> Option<u32> {
        if let Some(keycode) = self.charmap.get(&c) {
            Some(*keycode)
        } else {
            self.map_sym(c)
        }
    }
}

impl Drop for XConnection {
    fn drop(&mut self) {
        info!("Releasing all keys");
        for c in &self.held.clone() {
            self.press_symbol(*c, false).unwrap();
        }
        info!("Unbinding all keys");
        for keycode in self.charmap.values() {
            self.unbind_key(*keycode);
        }
        unsafe {
            XCloseDisplay(self.display);
        }
    }
}

impl DisplayOutput for XConnection {
    fn get_layout(&self) -> Result<String, DisplayOutputError> {
        // TODO: Better solution. https://unix.stackexchange.com/a/422493

        let result = Command::new("setxkbmap")
            .args(["-query"])
            .output()
            .expect("Failed to exec setxkbmap");
        let output = String::from_utf8_lossy(&result.stdout);
        let mut map = output
            .lines()
            .map(|l| l.split(':'))
            .map(|mut kv| (kv.next().unwrap_or(""), kv.next().unwrap_or("")));
        let layout = map
            .find(|(k, _): &(&str, &str)| *k == "layout")
            .map(|(_, v)| v.trim())
            .unwrap_or("");
        Ok(layout.to_string())
    }

    fn set_layout(&self, layout: &str) -> Result<(), DisplayOutputError> {
        Command::new("setxkbmap").args([layout]).output().unwrap();
        Ok(())
    }

    fn type_string(&mut self, string: &str) -> Result<(), DisplayOutputError> {
        let mut keycodes = Vec::with_capacity(string.len());

        // Make sure we have a keysym for every key
        for c in string.chars() {
            if c == '\0' {
                continue;
            }
            if let Some(keycode) = self.get_sym(c) {
                debug!("Type {} => {:x?}", keycode, c);
                keycodes.push(keycode);
            } else {
                error!("Could not allocate a keysym for unicode '{}'", c);
                return Err(DisplayOutputError::AllocationFailed(c));
            }
        }

        // Send keypresses in chunks
        // We have to increase the delay for each chunk (1ms intervals) as we can queue up events
        // much quicker than 1ms.
        // e.g. The tall moose
        // Instance 1   Press: 'The tall' # Stops at the 2nd space
        // Instance 1 Release: 'The tall'
        // Instance 2   Press: ' mo'      # Double o so we have to stop again
        // Instance 2 Release: ' mo'
        // Instance 3   Press: 'ose'
        // Instance 3 Release: 'ose'
        let mut keysym_queue = vec![];
        for k in keycodes.iter() {
            if !keysym_queue.contains(k) {
                keysym_queue.push(*k)
            } else {
                // Press/Release
                for qk in keysym_queue {
                    self.press_release_key(qk);
                }

                // Clear queue and insert the keysym we weren't able to add
                keysym_queue = vec![*k];
            }
        }

        // Handle remaining queue
        // Press/Release
        for qk in keysym_queue {
            self.press_release_key(qk);
        }

        unsafe {
            XFlush(self.display);
        }

        // Cleanup any symbols we had to map (just in case we need to remap new symbols another
        // time)
        for c in string.chars() {
            self.unmap_sym(c);
        }

        // Make sure to sleep the length of the delay to make sure we don't call this API again too
        // quickly and interfere with these keypresses.
        std::thread::sleep(std::time::Duration::from_millis(
            KEY_SEQUENCE_END_DELAY_MS + self.pending_delays as u64,
        ));

        Ok(())
    }

    fn press_symbol(&mut self, c: char, press: bool) -> Result<(), DisplayOutputError> {
        // Nothing to do
        if c == '\0' {
            return Ok(());
        }
        if let Some(keycode) = self.get_sym(c) {
            debug!("Set {:?} ({}) = {}", c, keycode, press);
            self.press_key(keycode, press);

            if press {
                self.held.push(c);
            } else {
                self.unmap_sym(c);
                self.held
                    .iter()
                    .position(|&x| x == c)
                    .map(|e| self.held.remove(e));
            }
            unsafe {
                XFlush(self.display);
            }
        }

        Ok(())
    }

    fn get_held(&mut self) -> Result<Vec<char>, DisplayOutputError> {
        Ok(self.held.clone())
    }

    fn set_held(&mut self, string: &str) -> Result<(), DisplayOutputError> {
        let s: Vec<char> = string.chars().collect();

        // This is a single instance, so update the pending delays first
        self.update_pending_delays();

        for c in &self.held.clone() {
            if !s.contains(c) {
                self.press_symbol(*c, false)?;
            }
        }
        for c in &s {
            self.press_symbol(*c, true)?;
        }

        // Make sure to sleep the length of the delay to make sure we don't call this API again too
        // quickly and interfere with these keypresses.
        std::thread::sleep(std::time::Duration::from_millis(
            KEY_SEQUENCE_END_DELAY_MS + self.pending_delays as u64,
        ));

        Ok(())
    }
}
