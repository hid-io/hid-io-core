/* Copyright (C) 2020-2022 by Jacob Alexander
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
use std::collections::{HashMap, VecDeque};

use std::convert::TryInto;
use std::io::{Seek, SeekFrom, Write};
use std::os::unix::io::IntoRawFd;
use std::time::Instant;
use tempfile::tempfile;

use wayland_client::{
    protocol::{wl_registry, wl_seat},
    Connection, Dispatch, EventQueue, QueueHandle,
};
use wayland_protocols_misc::zwp_virtual_keyboard_v1::client::zwp_virtual_keyboard_manager_v1;
use wayland_protocols_misc::zwp_virtual_keyboard_v1::client::zwp_virtual_keyboard_v1;

pub struct Key {
    pub keysym: xkbcommon::xkb::Keysym,
    pub keycode: u32,
    pub refcount: u32,
}

impl std::fmt::Debug for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "keysym:{} keycode:{} refcount:{}",
            self.keysym, self.keycode, self.refcount
        )
    }
}

pub struct Keymap {
    automatic_layout_regen: bool, // Automatically regenerate layout as needed on add() and remove()
    base_time: std::time::Instant,
    keysym_lookup: HashMap<char, Key>, // UTF-8 -> (keysym, keycode, refcount)
    unused_keycodes: VecDeque<u32>,    // Used to keep track of unused keycodes
    virtual_keyboard: zwp_virtual_keyboard_v1::ZwpVirtualKeyboardV1,
}

impl Keymap {
    pub fn new(
        virtual_keyboard: zwp_virtual_keyboard_v1::ZwpVirtualKeyboardV1,
        automatic_layout_regen: bool,
    ) -> Keymap {
        let keysym_lookup = HashMap::new();
        let base_time = Instant::now();

        // All keycodes are unused when initialized
        // Keycodes 8 -> 255 are valid and can be used
        let mut unused_keycodes: VecDeque<u32> = VecDeque::new();
        for n in 8..=255 {
            unused_keycodes.push_back(n);
        }

        Keymap {
            automatic_layout_regen,
            base_time,
            keysym_lookup,
            unused_keycodes,
            virtual_keyboard,
        }
    }

    /// Generates a single-level keymap.
    pub fn generate_keymap_string(&mut self) -> Result<String, DisplayOutputError> {
        let mut buf: Vec<u8> = Vec::new();
        writeln!(
            buf,
            "xkb_keymap {{

        xkb_keycodes \"hidio\" {{
            minimum = 8;
            maximum = 255;"
        )?;

        // Xorg can only consume up to 255 keys (this is handled by the keycode assignment)
        for (key, val) in self.keysym_lookup.iter() {
            write!(
                buf,
                "
            <I{}> = {}; // {}",
                val.keycode, val.keycode, key,
            )?;
        }

        // Setup the indicators (unused, but needed for Xwayland)
        writeln!(
            buf,
            "
            indicator 1 = \"Caps Lock\"; // Needed for Xwayland
        }};

        xkb_symbols \"hidio\" {{"
        )?;

        // NOTE (HaaTa): Tab and Return do not behave well as U<codepoint> keysyms
        //               Specify the names manually instead.
        for (key, val) in self.keysym_lookup.iter() {
            match key {
                '\n' => {
                    write!(
                        buf,
                        "
            key <I{}> {{ [ Return ] }}; // \\n",
                        val.keycode,
                    )?;
                }
                '\t' => {
                    write!(
                        buf,
                        "
            key <I{}> {{ [ Tab ] }}; // \\t",
                        val.keycode,
                    )?;
                }
                _ => {
                    write!(
                        buf,
                        "
            key <I{}> {{ [ U{:X} ] }}; // {}",
                        val.keycode,
                        val.keysym & 0x1F_FFFF, // XXX (HaaTa): I suspect there's a UTF-8 -> Keysym incompatibility for higher orders
                        //              this mask seems allow mappings to work
                        //              correctly but I don't think it's correct.
                        // Might be related to: https://docs.rs/xkbcommon/0.4.0/xkbcommon/xkb/type.Keysym.html
                        key,
                    )?;
                }
            }
        }

        writeln!(
            buf,
            "
        }};

        xkb_types \"hidio\" {{
            virtual_modifiers HidIo; // No modifiers, needed by Xorg.

            // These names are needed for Xwayland.
            type \"ONE_LEVEL\" {{
                modifiers= none;
                level_name[Level1]= \"Any\";
            }};
            type \"TWO_LEVEL\" {{
                level_name[Level1]= \"Base\";
            }};
            type \"ALPHABETIC\" {{
                level_name[Level1]= \"Base\";
            }};
            type \"KEYPAD\" {{
                level_name[Level1]= \"Base\";
            }};
            type \"SHIFT+ALT\" {{
                level_name[Level1]= \"Base\";
            }};

        }};

        xkb_compatibility \"hidio\" {{
            // Needed for Xwayland.
            interpret Any+AnyOf(all) {{
                action= SetMods(modifiers=modMapMods,clearLocks);
            }};
        }};
    }};"
        )?;

        String::from_utf8(buf).map_err(DisplayOutputError::Utf)
    }

    /// Apply XKB layout to virtual keyboard
    /// The layout is an XKB layout as a string (see generate_keymap_string())
    /// NOTE: This function does not flush any messages to Wayland, you'll need to schedule
    /// afterwards
    pub fn apply_layout(&mut self, layout: String) -> Result<(), DisplayOutputError> {
        // We need to build a file with a fd in order to pass the layout file to Wayland for
        // processing
        let keymap_size = layout.len();
        let keymap_size_u32: u32 = keymap_size.try_into().unwrap(); // Convert it from usize to u32, panics if it is not possible
        let keymap_size_u64: u64 = keymap_size.try_into().unwrap(); // Convert it from usize to u64, panics if it is not possible
        let mut keymap_file = tempfile().expect("Unable to create tempfile");

        // Allocate space in the file first
        keymap_file.seek(SeekFrom::Start(keymap_size_u64)).unwrap();
        keymap_file.write_all(&[0]).unwrap();
        keymap_file.rewind().unwrap();
        let mut data = unsafe {
            memmap2::MmapOptions::new()
                .map_mut(&keymap_file)
                .expect("Could not access data from memory mapped file")
        };
        data[..layout.len()].copy_from_slice(layout.as_bytes());

        // Get fd to pass to Wayland
        let keymap_raw_fd = keymap_file.into_raw_fd();
        self.virtual_keyboard
            .keymap(1, keymap_raw_fd, keymap_size_u32);
        Ok(())
    }

    /// Lookup keysym from a UTF-8 symbol
    /// \n and \t are special symbols for Return and Tab respectively
    pub fn lookup_sym(c: char) -> Option<xkbcommon::xkb::Keysym> {
        // Special character lookup, otherwise normal lookup
        let keysym = match c {
            '\n' => xkbcommon::xkb::keysyms::KEY_Return,
            '\t' => xkbcommon::xkb::keysyms::KEY_Tab,
            _ => {
                // Convert UTF-8 to a code point first to do the keysym lookup
                let codepoint = format!("U{:X}", c as u32);
                xkbcommon::xkb::keysym_from_name(&codepoint, xkbcommon::xkb::KEYSYM_NO_FLAGS)
            }
        };
        trace!("{} {:04X} -> U{:04X}", c, c as u32, keysym);

        // Make sure the keysym is valid
        if keysym != xkbcommon::xkb::keysyms::KEY_NoSymbol {
            Some(keysym)
        } else {
            None
        }
    }

    /// Adds UTF-8 symbols to be added to the virtual keyboard.
    /// Returns list of keycodes mapped, 1-to-1 mapping to the given vector for UTF-8 characters
    /// If any of the symbols could not be mapped, none of the symbols will mapped.
    /// Will increment a reference counter if the symbol has already been added.
    ///
    /// Will handle \n (Return) and \t (Tab) as special characters
    pub fn add(&mut self, chars: std::str::Chars) -> Result<Vec<u32>, DisplayOutputError> {
        let mut keysym_pairs: Vec<(char, xkbcommon::xkb::Keysym)> = Vec::new();
        let mut keycode_sequence: Vec<u32> = Vec::new();
        let mut regenerate = false;
        trace!("add({:?})", chars);

        // Lookup each of the keysyms
        for c in chars.clone() {
            if let Some(keysym) = Keymap::lookup_sym(c) {
                keysym_pairs.push((c, keysym));
            } else {
                return Err(DisplayOutputError::AllocationFailed(c));
            }
        }

        // Increment the reference counters and allocate keycodes
        for (c, keysym) in keysym_pairs {
            if self.keysym_lookup.contains_key(&c) {
                // Keycode already allocate, just increment refcount
                self.keysym_lookup.get_mut(&c).unwrap().refcount += 1;

                // Lookup keycode
                keycode_sequence.push(self.keysym_lookup.get(&c).unwrap().keycode);
                continue;
            }

            // Allocate keycode
            let keycode = if let Some(keycode) = self.unused_keycodes.pop_front() {
                keycode
            } else {
                error!("No more keycodes available! Check incoming sequence or held keys.");
                return Err(DisplayOutputError::AllocationFailed(c));
            };
            // Insert keysym and keycode for lookup
            self.keysym_lookup.insert(
                c,
                Key {
                    keysym,
                    keycode,
                    refcount: 1,
                },
            );

            // Setup output keycode sequence
            keycode_sequence.push(keycode);

            // Trigger a regen of the layout
            regenerate = true;
        }

        // Regenerate layout if necessary
        if regenerate && self.automatic_layout_regen {
            let layout = self.generate_keymap_string()?;
            trace!("add({:?}) regenerate {}", chars, layout);
            self.apply_layout(layout)?;
        }

        Ok(keycode_sequence)
    }

    /// Removes UTF-8 symbols from the virtual keyboard.
    /// Will decrement a reference counter and will only return zero if that symbols reference
    /// counter has reached zero.
    pub fn remove(&mut self, chars: std::str::Chars) -> Result<(), DisplayOutputError> {
        let mut regenerate = false;
        trace!("remove({:?})", chars);

        // Lookup each of the keysyms, decrementing the reference counters
        for c in chars {
            if self.keysym_lookup.contains_key(&c) {
                self.keysym_lookup.get_mut(&c).unwrap().refcount -= 1;
                // If we've exhausted the reference counter, remove the item
                let key = self.keysym_lookup.entry(c).or_insert(Key {
                    keysym: 0,
                    keycode: 0,
                    refcount: 0,
                });
                if key.refcount == 0 {
                    // Add the keycode back to the queue
                    self.unused_keycodes.push_back(key.keycode);

                    // Remove the entry
                    self.keysym_lookup.remove(&c);

                    // Trigger a regen of the layout
                    regenerate = true;
                }
            }
        }

        // Regenerate layout if necessary
        if regenerate && self.automatic_layout_regen {
            let layout = self.generate_keymap_string()?;
            self.apply_layout(layout)?;
        }

        Ok(())
    }

    /// Retrieves the keycode for a given if it exists
    /// Use add() if you're unsure if a keycode hasn't been mapped to a UTF-8 character yet
    pub fn get(&mut self, c: char) -> Option<&Key> {
        if self.keysym_lookup.contains_key(&c) {
            Some(self.keysym_lookup.entry(c).or_insert(Key {
                keysym: 0,
                keycode: 0,
                refcount: 0,
            }))
        } else {
            None
        }
    }

    /// Used to apply ms timestamps for Wayland key events
    fn get_time(&self) -> u32 {
        let duration = self.base_time.elapsed();
        let time = duration.as_millis();
        time.try_into().unwrap()
    }

    /// Press/Release a specific UTF-8 symbol
    /// NOTE: This function does not synchronize the event queue, should be done immediately after
    /// calling (unless you're trying to optimize scheduling).
    pub fn press_key(&mut self, c: char, press: bool) -> Result<(), DisplayOutputError> {
        let time = self.get_time();
        let state = u32::from(press);
        let keycode = if let Some(key) = self.keysym_lookup.get(&c) {
            // Adjust by 8, per xkb/xwayland requirements
            key.keycode - 8
        } else {
            return Err(DisplayOutputError::NoKeycode);
        };
        debug!("time:{} keycode:{}:{} state:{}", time, c, keycode, state);

        // Send key event message
        self.virtual_keyboard.key(time, keycode, state);
        Ok(())
    }

    /// Press then release a specific UTF-8 symbol
    /// Faster than individual calls to press_key as you don't need a delay (or sync) between press and
    /// release of the same keycode.
    /// NOTE: This function does not synchronize the event queue, should be done immediately after
    /// calling (unless you're trying to optimize scheduling).
    pub fn press_release_key(&mut self, c: char) -> Result<(), DisplayOutputError> {
        let time = self.get_time();
        let keycode = if let Some(key) = self.keysym_lookup.get(&c) {
            // Adjust by 8, per xkb/xwayland requirements
            key.keycode - 8
        } else {
            return Err(DisplayOutputError::NoKeycode);
        };
        debug!("time:{} keycode:{}:{}", time, c, keycode);

        // Send key event message
        self.virtual_keyboard.key(time, keycode, 1);
        self.virtual_keyboard.key(time, keycode, 0);
        Ok(())
    }
}

pub struct WaylandConnection {
    _conn: Connection,
    event_queue: EventQueue<VirtKbdState>,
    state: VirtKbdState,
    held: Vec<char>,
    keymap: Keymap,
}

struct VirtKbdState {
    keyboard_manager: Option<zwp_virtual_keyboard_manager_v1::ZwpVirtualKeyboardManagerV1>,
    seat: Option<wl_seat::WlSeat>,
}

impl VirtKbdState {
    fn new() -> Self {
        Self {
            keyboard_manager: None,
            seat: None,
        }
    }
}

impl Dispatch<wl_registry::WlRegistry, ()> for VirtKbdState {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        // When receiving events from the wl_registry, we are only interested in the
        // `global` event, which signals a new available global.
        // When receiving this event, we just print its characteristics in this example.
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            trace!("[{}] {} (v{})", name, interface, version);

            match &interface[..] {
                "wl_seat" => {
                    // Setup seat for keyboard
                    let seat = registry.bind::<wl_seat::WlSeat, _, _>(name, 1, qh, ());
                    state.seat = Some(seat);
                }
                "zwp_virtual_keyboard_manager_v1" => {
                    let manager = registry
                        .bind::<zwp_virtual_keyboard_manager_v1::ZwpVirtualKeyboardManagerV1, _, _>(
                        name,
                        1,
                        qh,
                        (),
                    );
                    state.keyboard_manager = Some(manager);
                }
                _ => {}
            }
        }
    }
}

impl Dispatch<zwp_virtual_keyboard_manager_v1::ZwpVirtualKeyboardManagerV1, ()> for VirtKbdState {
    fn event(
        _state: &mut Self,
        _manager: &zwp_virtual_keyboard_manager_v1::ZwpVirtualKeyboardManagerV1,
        event: zwp_virtual_keyboard_manager_v1::Event,
        _: &(),
        _: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        info!("Got a virtual keyboard manager event {:?}", event);
    }
}

impl Dispatch<zwp_virtual_keyboard_v1::ZwpVirtualKeyboardV1, ()> for VirtKbdState {
    fn event(
        _state: &mut Self,
        _manager: &zwp_virtual_keyboard_v1::ZwpVirtualKeyboardV1,
        event: zwp_virtual_keyboard_v1::Event,
        _: &(),
        _: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        info!("Got a virtual keyboard event {:?}", event);
    }
}

impl Dispatch<wl_seat::WlSeat, ()> for VirtKbdState {
    fn event(
        _: &mut Self,
        _: &wl_seat::WlSeat,
        event: wl_seat::Event,
        _: &(),
        _: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        info!("Got a seat event {:?}", event);
    }
}

impl WaylandConnection {
    pub fn new() -> Result<WaylandConnection, DisplayOutputError> {
        let held = Vec::new();

        // Setup Wayland Connection
        let conn = Connection::connect_to_env();

        // Make sure we made a connection
        let conn = match conn {
            Ok(conn) => conn,
            Err(e) => {
                error!("Failed to connect to Wayland");
                return Err(DisplayOutputError::Connection(e.to_string()));
            }
        };

        // Check to see if there was an error trying to connect
        if let Some(err) = conn.protocol_error() {
            error!(
                "Unknown Wayland initialization failure: {} {} {} {}",
                err.code, err.object_id, err.object_interface, err.message
            );
            return Err(DisplayOutputError::General(err.to_string()));
        }

        // Create the event queue
        let mut event_queue = conn.new_event_queue();
        // Get queue handle
        let qh = event_queue.handle();

        // Start registry
        let display = conn.display();
        display.get_registry(&qh, ());

        // Dispatch events so we can setup a virtual keyboard
        // This requires a wl_seat and zwp_virtual_keyboard_manager_v1
        let mut state = VirtKbdState::new();
        event_queue.roundtrip(&mut state).unwrap();

        // Setup Virtual Keyboard
        let seat = state.seat.as_ref().unwrap();
        let vk_mgr = state.keyboard_manager.as_ref().unwrap();
        let virtual_keyboard = vk_mgr.create_virtual_keyboard(seat, &qh, ());

        // Setup Keymap
        let keymap = Keymap::new(virtual_keyboard, true);

        Ok(WaylandConnection {
            _conn: conn,
            event_queue,
            state,
            held,
            keymap,
        })
    }
}

impl Drop for WaylandConnection {
    fn drop(&mut self) {
        warn!("Releasing and unbinding all keys");
        for c in self.held.iter() {
            self.keymap.press_key(*c, false).unwrap();
            self.keymap.remove(c.to_string().chars()).unwrap();
        }
    }
}

impl DisplayOutput for WaylandConnection {
    fn get_layout(&self) -> Result<String, DisplayOutputError> {
        warn!("Unimplemented get_layout()");
        Err(DisplayOutputError::Unimplemented)
    }
    fn set_layout(&self, _layout: &str) -> Result<(), DisplayOutputError> {
        warn!("Unimplemented set_layout()");
        Err(DisplayOutputError::Unimplemented)
    }

    /// Type the given UTF-8 string using the virtual keyboard
    /// Should behave nicely even if keys were previously held (those keys will continue to be held
    /// after sequence is complete, though there may be some issues with this case due to the
    /// layout switching)
    fn type_string(&mut self, string: &str) -> Result<(), DisplayOutputError> {
        // Allocate keysyms to virtual keyboard layout
        self.keymap.add(string.chars())?;

        for c in string.chars() {
            self.keymap.press_release_key(c)?;
        }

        // Pump event queue
        self.event_queue.roundtrip(&mut self.state).unwrap();

        // Deallocate keysyms in virtual keyboard layout
        self.keymap.add(string.chars())?;

        Ok(())
    }

    /// Press/Release a given UTF-8 symbol
    /// NOTE: This function does not synchronize the event queue, should be done immediately after
    /// calling (unless you're trying to optimize scheduling).
    fn press_symbol(&mut self, c: char, press: bool) -> Result<(), DisplayOutputError> {
        // Nothing to do
        if c == '\0' {
            return Ok(());
        }

        if press {
            self.keymap.add(c.to_string().chars())?;
            self.keymap.press_key(c, true)?;
            self.held.push(c);
        } else {
            self.keymap.press_key(c, false)?;
            self.held
                .iter()
                .position(|&x| x == c)
                .map(|e| self.held.remove(e));
            self.keymap.remove(c.to_string().chars())?;
        }

        Ok(())
    }

    /// Retrieve currently held UTF-8 symbols
    fn get_held(&mut self) -> Result<Vec<char>, DisplayOutputError> {
        Ok(self.held.clone())
    }

    /// Set keys to hold
    /// If any UTF-8 chars are not included from the previous invocation they will be released
    fn set_held(&mut self, string: &str) -> Result<(), DisplayOutputError> {
        let s: Vec<char> = string.chars().collect();

        for c in &self.held.clone() {
            if !s.contains(c) {
                self.press_symbol(*c, false)?;
            }
        }
        for c in &s {
            self.press_symbol(*c, true)?;
        }

        // Pump event queue
        self.event_queue.roundtrip(&mut self.state).unwrap();
        Ok(())
    }
}

// ------- Test Cases -------

#[cfg(test)]
mod test {
    use super::*;
    use crate::logging::setup_logging_lite;

    // This test will fail unless you have access to wayland
    #[test]
    #[ignore]
    fn keymap_basic_test() {
        setup_logging_lite().ok();

        // Setup Wayland Connection
        let conn = Connection::connect_to_env();

        // Make sure we made a connection
        let conn = match conn {
            Ok(conn) => conn,
            Err(e) => {
                panic!("Failed to connect to Wayland {}", e);
            }
        };

        // Check to see if there was an error trying to connect
        if let Some(err) = conn.protocol_error() {
            panic!(
                "Unknown Wayland initialization failure: {} {} {} {}",
                err.code, err.object_id, err.object_interface, err.message
            );
        }

        // Create the event queue
        let mut event_queue = conn.new_event_queue();
        // Get queue handle
        let qh = event_queue.handle();

        // Start registry
        let display = conn.display();
        display.get_registry(&qh, ());

        // Dispatch events so we can setup a virtual keyboard
        // This requires a wl_seat and zwp_virtual_keyboard_manager_v1
        let mut vkstate = VirtKbdState::new();
        event_queue.roundtrip(&mut vkstate).unwrap();

        // Setup Virtual Keyboard
        let seat = vkstate.seat.as_ref().unwrap();
        let vk_mgr = vkstate.keyboard_manager.as_ref().unwrap();
        let virtual_keyboard = vk_mgr.create_virtual_keyboard(&seat, &qh, ());

        // Setup Keymap for tests
        let mut keymap = Keymap::new(virtual_keyboard, false);

        keymap.add("abc".chars()).unwrap();
        let layout = keymap.generate_keymap_string().unwrap();
        info!("{}", layout);

        // Validate layout
        let xkb_context = xkbcommon::xkb::Context::new(xkbcommon::xkb::CONTEXT_NO_FLAGS);
        let xkb_keymap = xkbcommon::xkb::Keymap::new_from_string(
            &xkb_context,
            layout,
            xkbcommon::xkb::KEYMAP_FORMAT_TEXT_V1,
            xkbcommon::xkb::KEYMAP_COMPILE_NO_FLAGS,
        )
        .expect("Failed to create keymap");
        let state = xkbcommon::xkb::State::new(&xkb_keymap);
        assert_eq!(state.key_get_one_sym(8), xkbcommon::xkb::KEY_a);
        assert_eq!(state.key_get_one_sym(9), xkbcommon::xkb::KEY_b);
        assert_eq!(state.key_get_one_sym(10), xkbcommon::xkb::KEY_c);

        // Validate the 'b' symbol keycode and refcount
        assert!(keymap.get('b').unwrap().keycode == 9);
        assert!(keymap.get('b').unwrap().refcount == 1);
        keymap.remove("abc".chars()).unwrap();

        // Validate the 'b' symbol keycode and refcount again (after removing once)
        keymap.add("b".chars()).unwrap();
        keymap.add("b".chars()).unwrap();
        assert!(keymap.get('b').unwrap().keycode == 11); // A new keycode should be allocated compared to the previous test
        assert!(keymap.get('b').unwrap().refcount == 2);

        // Add more complicated symbols
        keymap.add("z🙊🐇🦜".chars()).unwrap();
        let layout = keymap.generate_keymap_string().unwrap();
        info!("{}", layout);

        // Validate layout
        let xkb_context = xkbcommon::xkb::Context::new(xkbcommon::xkb::CONTEXT_NO_FLAGS);
        let xkb_keymap = xkbcommon::xkb::Keymap::new_from_string(
            &xkb_context,
            layout.clone(),
            xkbcommon::xkb::KEYMAP_FORMAT_TEXT_V1,
            xkbcommon::xkb::KEYMAP_COMPILE_NO_FLAGS,
        )
        .expect("Failed to create keymap");
        let state = xkbcommon::xkb::State::new(&xkb_keymap);

        assert_eq!(state.key_get_one_sym(11), xkbcommon::xkb::KEY_b);
        assert_eq!(state.key_get_one_sym(12), xkbcommon::xkb::KEY_z);
        assert_eq!(state.key_get_one_sym(13), Keymap::lookup_sym('🙊').unwrap());
        assert_eq!(state.key_get_one_sym(14), Keymap::lookup_sym('🐇').unwrap());
        assert_eq!(state.key_get_one_sym(15), Keymap::lookup_sym('🦜').unwrap());

        // Attempt to apply layout
        keymap.apply_layout(layout).unwrap();
        event_queue.roundtrip(&mut vkstate).unwrap();
    }
}
