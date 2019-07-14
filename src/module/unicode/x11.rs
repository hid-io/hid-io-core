/* Copyright (C) 2019 by Jacob Alexander
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

use std::collections::HashMap;
use std::ffi::CString;
use std::os::raw::{c_int, c_uchar, c_void};
use std::process::Command;
use std::ptr::null;
use std::{thread, time};
use x11::xlib::*;
use x11::xtest::*;

use crate::module::unicode::UnicodeOutput;

const KEY_DELAY_US: u64 = 60000;

pub struct XConnection {
    display: *mut x11::xlib::_XDisplay,
    charmap: HashMap<char, u32>,
    held: Vec<char>,
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
            XConnection {
                display,
                charmap,
                held,
            }
        }
    }

    #[link(name = "X11")]
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
        unsafe {
            let xs = CString::new(s);
            XStringToKeysym(xs.unwrap().as_ptr())
        }
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

    pub fn press_key(&self, keycode: u32, state: bool, time: x11::xlib::Time) {
        unsafe {
            XTestFakeKeyEvent(self.display, keycode, state as i32, time);
        }
    }

    pub fn map_sym(&mut self, c: char) -> Option<u32> {
        let keysym = XConnection::lookup_sym(c);
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
            self.press_symbol(*c, false);
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

impl UnicodeOutput for XConnection {
    fn get_layout(&self) -> String {
        // TODO: Better solution. https://unix.stackexchange.com/a/422493

        let result = Command::new("setxkbmap")
            .args(&["-query"])
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
        layout.to_string()
    }

    fn set_layout(&self, layout: &str) {
        Command::new("setxkbmap").args(&[layout]).output().unwrap();
    }

    fn type_string(&mut self, string: &str) {
        let mut keycodes = Vec::with_capacity(string.len());

        for c in string.chars() {
            if c == '\0' {
                continue;
            }
            if let Some(keycode) = self.get_sym(c) {
                info!("Type {} => {:x?}", keycode, c);
                keycodes.push(keycode);
            }
        }

        let time = x11::xlib::CurrentTime;
        for k in keycodes.iter() {
            self.press_key(*k, true, time);
            thread::sleep(time::Duration::from_micros(KEY_DELAY_US));
            self.press_key(*k, false, time);
            //thread::sleep(time::Duration::from_micros(KEY_DELAY_US));
        }

        unsafe {
            XFlush(self.display);
        }

        for c in string.chars() {
            self.unmap_sym(c);
        }
    }

    fn press_symbol(&mut self, c: char, press: bool) {
        if c == '\0' {
            return;
        }
        if let Some(keycode) = self.get_sym(c) {
            println!("Set {:?} ({}) = {}", c, keycode, press);
            self.press_key(keycode, press, x11::xlib::CurrentTime);

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
    }

    fn get_held(&mut self) -> Vec<char> {
        self.held.clone()
    }

    fn set_held(&mut self, string: &str) {
        let s: Vec<char> = string.chars().collect();
        for c in &self.held.clone() {
            if !s.contains(c) {
                self.press_symbol(*c, false);
            }
        }
        for c in &s {
            self.press_symbol(*c, true);
        }
    }
}
