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

use core_graphics::event::CGEvent;
use core_graphics::event_source::CGEventSource;
use core_graphics::event_source::CGEventSourceStateID::HIDSystemState;

use crate::module::displayserver::{DisplayOutput, DisplayOutputError};

#[allow(dead_code)]
pub struct QuartzConnection {
    charmap: HashMap<char, u32>,
    held: Vec<char>,
}

impl Default for QuartzConnection {
    fn default() -> Self {
        Self::new()
    }
}

impl QuartzConnection {
    pub fn new() -> QuartzConnection {
        let charmap = HashMap::new();
        let held = Vec::new();
        QuartzConnection { charmap, held }
    }

    pub fn press_key(&self, c: char, state: bool) {
        use core_graphics::event::CGEventTapLocation;
        let source = CGEventSource::new(HIDSystemState).unwrap();

        let mut buf = [0; 2];
        let event = CGEvent::new_keyboard_event(source, 0, state).unwrap();
        event.set_string_from_utf16_unchecked(c.encode_utf16(&mut buf));
        event.post(CGEventTapLocation::HID);
    }

    pub fn press_keycode(&self, keycode: core_graphics::event::CGKeyCode, state: bool) {
        use core_graphics::event::CGEventTapLocation;
        let source = CGEventSource::new(HIDSystemState).unwrap();

        let event = CGEvent::new_keyboard_event(source, keycode, state).unwrap();
        event.post(CGEventTapLocation::HID);
    }

    pub fn type_utf8(&self, string: &str) {
        use core_graphics::event::CGEventTapLocation;
        let source = CGEventSource::new(HIDSystemState).unwrap();

        // Press
        let event = CGEvent::new_keyboard_event(source.clone(), 0, true).unwrap();
        event.set_string(string);
        event.post(CGEventTapLocation::HID);

        // Release
        let event = CGEvent::new_keyboard_event(source, 0, false).unwrap();
        event.set_string(string);
        event.post(CGEventTapLocation::HID);
    }
}

impl Drop for QuartzConnection {
    fn drop(&mut self) {
        info!("Releasing all keys");
        for c in &self.held.clone() {
            self.press_symbol(*c, false).unwrap();
        }
    }
}

impl DisplayOutput for QuartzConnection {
    fn get_layout(&self) -> Result<String, DisplayOutputError> {
        warn!("Unimplemented");
        Err(DisplayOutputError {})
    }

    fn set_layout(&self, _layout: &str) -> Result<(), DisplayOutputError> {
        warn!("Unimplemented");
        Err(DisplayOutputError {})
    }

    /// Types a UTF-8 string into the focused window
    /// Will handle special characters \n and \t to be Return and Tab respectively
    fn type_string(&mut self, string: &str) -> Result<(), DisplayOutputError> {
        // Splice the string into chunks split by \n and \t which need to be handled
        // separately. macOS seems to handle double characters well with this method
        // so it's not necessary to take those into account (unlike x11).
        // Unfortunately, double UTF-8 emojis seem to die a horrible death so we need
        // to implement chunking anyways for double characters...oh well.
        let mut queue = vec![];
        for c in string.chars() {
            match c {
                '\n' | '\t' => {
                    // If there were characters before, print now
                    if !queue.is_empty() {
                        let chunk: String = queue.into_iter().collect();
                        self.type_utf8(&chunk);
                        queue = vec![];
                    }

                    // Lookup keycode
                    let keycode = match c {
                        '\n' => core_graphics::event::KeyCode::RETURN,
                        '\t' => core_graphics::event::KeyCode::TAB,
                        _ => {
                            continue;
                        }
                    };

                    // Press/release special key
                    self.press_keycode(keycode, true);
                    self.press_keycode(keycode, false);
                }
                _ => {
                    // Check if we've already queued up this symbol
                    // Push the current queue if there's a duplicate
                    if queue.contains(&c) {
                        let chunk: String = queue.into_iter().collect();
                        self.type_utf8(&chunk);
                        queue = vec![];
                    }
                    queue.push(c);
                }
            }
        }

        // Print final chunk of string
        if !queue.is_empty() {
            let chunk: String = queue.into_iter().collect();
            self.type_utf8(&chunk);
        }

        Ok(())
    }

    fn press_symbol(&mut self, c: char, press: bool) -> Result<(), DisplayOutputError> {
        self.press_key(c, press);
        Ok(())
    }

    fn get_held(&mut self) -> Result<Vec<char>, DisplayOutputError> {
        Ok(self.held.clone())
    }

    fn set_held(&mut self, string: &str) -> Result<(), DisplayOutputError> {
        // TODO (HaaTa)
        // - Get system repeat rate
        // - Use an event thread to wake-up per
        //   * Initial repeat
        //   * Repeat speed
        // - Send most recently held key
        let s: Vec<char> = string.chars().collect();
        for c in &self.held.clone() {
            if !s.contains(c) {
                self.press_symbol(*c, false).unwrap();
            }
        }
        for c in &s {
            self.press_symbol(*c, true).unwrap();
        }

        Ok(())
    }
}
