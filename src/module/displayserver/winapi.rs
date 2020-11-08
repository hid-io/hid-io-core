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

use crate::module::displayserver::{DisplayOutput, DisplayOutputError};

use winapi::ctypes::c_int;
use winapi::um::winuser;

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

    pub fn press_key(&self, c: char, state: bool) {
        let flags = if state {
            winuser::KEYEVENTF_UNICODE // Defaults to down
        } else {
            winuser::KEYEVENTF_UNICODE | winuser::KEYEVENTF_KEYUP
        };

        // Handle converting UTF-8 to UTF-16
        // Since UTF-16 doesn't handle the longer 32-bit characters
        // the value is encoded into high and low surrogates and must
        // be sent in succession without a "keyup"
        // Sequence taken from enigo
        // (https://github.com/enigo-rs/enigo/blob/f8d6dea72d957c693ee65c3b6bf5b15afbc4858e/src/win/win_impl.rs#L109)
        let mut buffer = [0; 2];
        let result = c.encode_utf16(&mut buffer);
        if result.len() == 1 {
            self.keyboard_event(flags, 0, result[0]);
        } else {
            for utf16_surrogate in result {
                self.keyboard_event(flags, 0, *utf16_surrogate);
            }
        }
    }

    /// Sends a keyboard event
    /// Used for Unicode in this module, but can be used for normal virtual keycodes and scancodes
    /// as well.
    fn keyboard_event(&self, flags: u32, vk: u16, scan: u16) {
        let mut event = winuser::INPUT {
            type_: winuser::INPUT_KEYBOARD,
            u: unsafe {
                std::mem::transmute_copy(&winuser::KEYBDINPUT {
                    wVk: vk,     // Virtual-Key Code (must be 0 when sending Unicode)
                    wScan: scan, // Hardware scancode (set to utf16 value when Unicode, must send surrogate pairs separately if larger than u16)
                    dwFlags: flags,
                    // KEYEVENTF_EXTENDEDKEY
                    // KEYEVENTF_KEYUP
                    // KEYEVENTF_SCANCODE
                    // KEYEVENTF_UNICODE
                    // Unset
                    time: 0,        // TODO Can we manipulate this for lower latency?
                    dwExtraInfo: 0, // ???
                })
            },
        };
        unsafe {
            winuser::SendInput(
                1,
                &mut event as winuser::LPINPUT,
                size_of::<winuser::INPUT>() as c_int,
            )
        };
    }

    #[allow(dead_code)]
    /// Retrieves the keyboard delay from HKEY_CURRENT_USER\Control Panel\Keyboard\KeyboardDelay
    /// KeyboardDelay can be from 0-3
    /// 0 - 250 ms - Shortest
    /// 1 - 500 ms - Default
    /// 2 - 750 ms
    /// 3 -   1 s  - Longest
    fn keyboard_delay(&self) -> std::io::Result<std::time::Duration> {
        let hkcu = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
        let keyboard = hkcu.open_subkey("Control Panel\\Keyboard")?;
        let delay_val: String = keyboard.get_value("KeyboardDelay")?;
        let delay_val: u64 = delay_val.parse().unwrap();
        Ok(std::time::Duration::from_millis(250 + delay_val * 250))
    }

    #[allow(dead_code)]
    /// Retrieves the keyboard speed from HKEY_CURRENT_USER\Control Panel\Keyboard\KeyboardSpeed
    /// KeyboardSpeed can be from 0-31
    /// There are 32 levels (0-31) but the cps go from 2-30 (28 levels).
    /// This means that each setting level (28/31 == 0.9032258). We subtract 1 from 32 as we start
    /// at 2 cps on the first setting.
    /// Using microseconds to get more precision.
    ///
    /// 1_000_000 us / 2 cps + level * 8/7 = us per character
    ///
    /// cps - Characters per second
    /// 0  - 2 cps  - 500_000 us - Slowest
    /// ...
    /// 31 - 30 cps - 33_000 us - Fastest - Default
    fn keyboard_speed(&self) -> std::io::Result<std::time::Duration> {
        let hkcu = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
        let keyboard = hkcu.open_subkey("Control Panel\\Keyboard")?;
        let speed_val: String = keyboard.get_value("KeyboardSpeed")?;
        let speed_val: u64 = speed_val.parse().unwrap();
        let us_delay = 1_000_000 / (2 + speed_val * 28 / 31);
        Ok(std::time::Duration::from_micros(us_delay))
    }
}

impl Drop for DisplayConnection {
    fn drop(&mut self) {
        info!("Releasing all unicode keys");
        self.set_held("").unwrap();
    }
}

impl DisplayOutput for DisplayConnection {
    fn get_layout(&self) -> Result<String, DisplayOutputError> {
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
        Ok(layout.to_string())
    }

    fn set_layout(&self, layout: &str) -> Result<(), DisplayOutputError> {
        match Command::new("powershell")
            .args(&[
                "-Command",
                &format!("Set-WinUserLanguageList -Force '{}'", &layout),
            ])
            .output()
        {
            Ok(_) => Ok(()),
            Err(_e) => {
                error!("Could not set language");
                Err(DisplayOutputError {})
            }
        }
    }

    fn type_string(&mut self, string: &str) -> Result<(), DisplayOutputError> {
        for c in string.chars() {
            self.press_key(c, true);
            self.press_key(c, false);
        }
        Ok(())
    }

    fn press_symbol(&mut self, c: char, press: bool) -> Result<(), DisplayOutputError> {
        self.press_key(c, press);

        if press {
            self.held.push(c);
        } else {
            self.held
                .iter()
                .position(|&x| x == c)
                .map(|e| self.held.remove(e));
        }
        Ok(())
    }

    fn get_held(&mut self) -> Result<Vec<char>, DisplayOutputError> {
        Ok(self.held.clone())
    }

    fn set_held(&mut self, string: &str) -> Result<(), DisplayOutputError> {
        let s: Vec<char> = string.chars().collect();
        for c in &self.held.clone() {
            if !s.contains(c) {
                match self.press_symbol(*c, false) {
                    Ok(_) => {}
                    Err(e) => {
                        return Err(e);
                    }
                }
            }
        }
        for c in &s {
            match self.press_symbol(*c, true) {
                Ok(_) => {}
                Err(e) => {
                    return Err(e);
                }
            }
        }
        Ok(())
    }
}
