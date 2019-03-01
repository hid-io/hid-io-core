use std::ffi::CString;
use std::os::raw::{c_int, c_uchar, c_void};
use std::ptr::null;
use std::{thread, time};
use std::collections::HashMap;
use std::process::Command;

use core_graphics::event_source::CGEventSourceStateID::HIDSystemState;
use core_graphics::event_source::CGEventSource;
use core_graphics::event::{CGEvent, CGEventFlags, CGKeyCode};

use crate::module::unicode::UnicodeOutput;

const KEY_DELAY_US: u64 = 60000;

pub struct OSXConnection {
    charmap: HashMap<char, u32>,
    held: Vec<char>,
}

impl Default for OSXConnection {
	fn default() -> Self {
		Self::new()
	}
}

impl OSXConnection {
    pub fn new() -> OSXConnection {
        unsafe {
            let charmap = HashMap::new();
            let held = Vec::new();
            OSXConnection { charmap, held }
        }
    }

	pub fn press_key(&self, c: char, state: bool) {
	    use core_graphics::event::CGEventType::*;
	    use core_graphics::event::{CGEventTapLocation, CGEventType};
	    let source = CGEventSource::new(HIDSystemState).unwrap();

	    let mut buf = [0; 2];
	    let event = CGEvent::new_keyboard_event(source, 0, state).unwrap();
	    event.set_string_from_utf16_unchecked(c.encode_utf16(&mut buf));
	    event.post(CGEventTapLocation::HID);
	}
}

impl Drop for OSXConnection {
    fn drop(&mut self) {
	info!("Releasing all keys");
        for c in &self.held.clone() {
            self.press_symbol(*c, false);
        }
    }
}

impl UnicodeOutput for OSXConnection {
    fn get_layout(&self) -> String {
	warn!("Unimplemented");
	"".to_string()
    }

    fn set_layout(&self, layout: &str) {
	warn!("Unimplemented");
    }

    fn type_string(&mut self, string: &str) {
	for c in string.chars() {
		self.press_key(c, true);
		self.press_key(c, false);
	}
    }

    fn press_symbol(&mut self, c: char, press: bool) {
	self.press_key(c, press);
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
