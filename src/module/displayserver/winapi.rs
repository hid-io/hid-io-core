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

use std::collections::HashMap;
use std::mem::size_of;
use std::process::Command;

use crate::module::displayserver::DisplayOutput;

use winapi::ctypes::wchar_t;
//use winapi::um::{winnls, winnt, winuser};
use winapi::um::winuser;

//const KEY_DELAY_US: u64 = 60000;

#[allow(dead_code)]
pub struct DisplayConnection {
    charmap: HashMap<char, u32>,
    held: Vec<char>,
}

impl Default for DisplayConnection {
    fn default() -> Self {
        Self::new()
    }
}

impl DisplayConnection {
    pub fn new() -> DisplayConnection {
        let charmap = HashMap::new();
        let held = Vec::new();
        DisplayConnection { charmap, held }
    }

    pub fn press_key(&self, c: wchar_t, state: bool) {
        let flags = if state {
            winuser::KEYEVENTF_UNICODE // Defaults to down
        } else {
            winuser::KEYEVENTF_UNICODE | winuser::KEYEVENTF_KEYUP
        };

        let mut input = unsafe {
            let mut i: winuser::INPUT_u = std::mem::zeroed();
            let mut ki = i.ki_mut();
            ki.wScan = c;
            ki.dwFlags = flags;

            winuser::INPUT {
                type_: winuser::INPUT_KEYBOARD,
                u: i,
            }
        };
        unsafe {
            winuser::SendInput(1, &mut input, size_of::<winuser::INPUT>() as i32);
        }
    }
}

impl Drop for DisplayConnection {
    fn drop(&mut self) {
        info!("Releasing all unicode keys");
        self.set_held("");
    }
}

impl UnicodeOutput for DisplayConnection {
    fn get_layout(&self) -> String {
        let result = Command::new("powershell")
            .args(&["-Command", "Get-WinUserLanguageList"])
            .output()
            .expect("Failed to exec");
        let output = String::from_utf8_lossy(&result.stdout);
        let mut map = output
            .lines()
            .filter(|l| l.contains(':'))
            .map(|l| l.split(':'))
            .map(|mut kv| (kv.next().unwrap().trim(), kv.next().unwrap().trim()));
        let layout = map
            .find(|(k, _): &(&str, &str)| *k == "LanguageTag")
            .map(|(_, v)| v)
            .unwrap_or("");
        layout.to_string()
    }

    fn set_layout(&self, layout: &str) {
        match Command::new("powershell")
            .args(&[
                "-Command",
                &format!("Set-WinUserLanguageList -Force '{}'", &layout),
            ])
            .output()
        {
            Ok(_) => (),
            Err(_e) => panic!("Could not set language"),
        }
    }

    fn type_string(&mut self, string: &str) {
        for c in string.encode_utf16() {
            self.press_key(c, true);
        }
    }

    fn press_symbol(&mut self, c: char, press: bool) {
        let mut buff = [0; 2];
        c.encode_utf16(&mut buff);
        for k in buff.iter() {
            self.press_key(*k, press);
        }

        if press {
            self.held.push(c);
        } else {
            self.held
                .iter()
                .position(|&x| x == c)
                .map(|e| self.held.remove(e));
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
