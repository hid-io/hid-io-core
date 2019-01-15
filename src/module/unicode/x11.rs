use std::ffi::CString;
use std::os::raw::{c_int, c_uchar, c_void};
use std::ptr::null;
use std::{thread, time};
use x11::xlib::*;
use x11::xtest::*;

use crate::module::unicode::UnicodeOutput;

const KEY_DELAY_US: u64 = 60000;

pub struct XConnection {
    display: *mut x11::xlib::_XDisplay,
}

impl XConnection {
    pub fn new() -> XConnection {
        unsafe {
            let display = XOpenDisplay(null());
            XConnection { display }
        }
    }

    #[link(name = "X11")]
    pub fn find_unused_keycode(&self) -> u32 {
        let display = self.display;
        let mut keycode = 0;

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
                let mut empty = true;
                for j in 0..keysyms_per_keycode {
                    let symindex = (i - keycode_low) * keysyms_per_keycode + j;
                    if *keysyms.offset(symindex as isize) != 0 {
                        empty = false;
                        break;
                    }
                }
                if empty {
                    keycode = i as u32;
                    break;
                }
            }

            XFree(keysyms as *mut c_void);
        }

        keycode
    }

    pub fn bind_key(&self, keycode: u32, symbol: char) {
        let hex: u32 = symbol.into();
        let s = format!("U{:x}", hex);

        unsafe {
            let xs = CString::new(s);
            let mut sym = XStringToKeysym(xs.unwrap().as_ptr());
            XChangeKeyboardMapping(self.display, keycode as i32, 1, &mut sym, 1);
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
}

impl Drop for XConnection {
    fn drop(&mut self) {
        unsafe {
            XCloseDisplay(self.display);
        }
    }
}

impl UnicodeOutput for XConnection {
    fn type_string(&self, string: &str) {
        let mut keycodes = Vec::with_capacity(string.len());

        for c in string.chars() {
            let keycode = self.find_unused_keycode();
            self.bind_key(keycode, c);
            keycodes.push(keycode);
        }

        let time = x11::xlib::CurrentTime;
        for k in keycodes.iter() {
            self.press_key(*k, true, time);
            thread::sleep(time::Duration::from_micros(KEY_DELAY_US));
            self.press_key(*k, false, time);
            thread::sleep(time::Duration::from_micros(KEY_DELAY_US));
            unsafe {
                XFlush(self.display);
            }
        }

        for k in keycodes.iter() {
            self.unbind_key(*k);
        }
    }

    fn press_symbol(&self, c: char, state: bool) {
        let keycode = self.find_unused_keycode();
        self.bind_key(keycode, c);
        self.press_key(keycode, state, 0);
        self.unbind_key(keycode);
    }
}
