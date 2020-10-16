/* Copyright (C) 2020 by Jacob Alexander
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

use crate::api::Endpoint;
use crate::api::EvdevInfo;
use crate::common_capnp;
use crate::logging::setup_logging_lite;
use crate::mailbox;
use crate::module::vhid;
use crate::protocol::hidio;
use crate::RUNNING;
use evdev_rs;
use lazy_static::lazy_static;
use regex::Regex;
use std::sync::atomic::Ordering;
use std::time::Instant;

// TODO This should be converted to use hid-io/layouts (may need a rust package to handle
// conversion)
const EVDEV2HIDKEY: [(hidio::HIDIOCommandID, u16); 547] = [
    (hidio::HIDIOCommandID::HIDKeyboard, 0x00), // Reserved
    (hidio::HIDIOCommandID::HIDKeyboard, 0x29), // Esc
    (hidio::HIDIOCommandID::HIDKeyboard, 0x1E), // 1
    (hidio::HIDIOCommandID::HIDKeyboard, 0x1F), // 2
    (hidio::HIDIOCommandID::HIDKeyboard, 0x20), // 3
    (hidio::HIDIOCommandID::HIDKeyboard, 0x21), // 4
    (hidio::HIDIOCommandID::HIDKeyboard, 0x22), // 5
    (hidio::HIDIOCommandID::HIDKeyboard, 0x23), // 6
    (hidio::HIDIOCommandID::HIDKeyboard, 0x24), // 7
    (hidio::HIDIOCommandID::HIDKeyboard, 0x25), // 8
    (hidio::HIDIOCommandID::HIDKeyboard, 0x26), // 9
    (hidio::HIDIOCommandID::HIDKeyboard, 0x27), // 0
    (hidio::HIDIOCommandID::HIDKeyboard, 0x2D), // Minus
    (hidio::HIDIOCommandID::HIDKeyboard, 0x2E), // Equal
    (hidio::HIDIOCommandID::HIDKeyboard, 0x2A), // Backspace
    (hidio::HIDIOCommandID::HIDKeyboard, 0x2B), // Tab
    (hidio::HIDIOCommandID::HIDKeyboard, 0x14), // Q
    (hidio::HIDIOCommandID::HIDKeyboard, 0x1A), // W
    (hidio::HIDIOCommandID::HIDKeyboard, 0x08), // E
    (hidio::HIDIOCommandID::HIDKeyboard, 0x15), // R
    (hidio::HIDIOCommandID::HIDKeyboard, 0x17), // T
    (hidio::HIDIOCommandID::HIDKeyboard, 0x1C), // Y
    (hidio::HIDIOCommandID::HIDKeyboard, 0x18), // U
    (hidio::HIDIOCommandID::HIDKeyboard, 0x0C), // I
    (hidio::HIDIOCommandID::HIDKeyboard, 0x12), // O
    (hidio::HIDIOCommandID::HIDKeyboard, 0x13), // P
    (hidio::HIDIOCommandID::HIDKeyboard, 0x2F), // Left Bracket
    (hidio::HIDIOCommandID::HIDKeyboard, 0x30), // Right Bracket
    (hidio::HIDIOCommandID::HIDKeyboard, 0x28), // Enter
    (hidio::HIDIOCommandID::HIDKeyboard, 0xE0), // Left Control
    (hidio::HIDIOCommandID::HIDKeyboard, 0x04), // A
    (hidio::HIDIOCommandID::HIDKeyboard, 0x16), // S
    (hidio::HIDIOCommandID::HIDKeyboard, 0x07), // D
    (hidio::HIDIOCommandID::HIDKeyboard, 0x09), // F
    (hidio::HIDIOCommandID::HIDKeyboard, 0x0A), // G
    (hidio::HIDIOCommandID::HIDKeyboard, 0x0B), // H
    (hidio::HIDIOCommandID::HIDKeyboard, 0x0D), // J
    (hidio::HIDIOCommandID::HIDKeyboard, 0x0E), // K
    (hidio::HIDIOCommandID::HIDKeyboard, 0x0F), // L
    (hidio::HIDIOCommandID::HIDKeyboard, 0x33), // Semicolon
    (hidio::HIDIOCommandID::HIDKeyboard, 0x34), // Quote
    (hidio::HIDIOCommandID::HIDKeyboard, 0x35), // Backtick
    (hidio::HIDIOCommandID::HIDKeyboard, 0xE1), // Left Shift
    (hidio::HIDIOCommandID::HIDKeyboard, 0x31), // Backslash
    (hidio::HIDIOCommandID::HIDKeyboard, 0x1D), // Z
    (hidio::HIDIOCommandID::HIDKeyboard, 0x1B), // X
    (hidio::HIDIOCommandID::HIDKeyboard, 0x06), // C
    (hidio::HIDIOCommandID::HIDKeyboard, 0x19), // V
    (hidio::HIDIOCommandID::HIDKeyboard, 0x05), // B
    (hidio::HIDIOCommandID::HIDKeyboard, 0x11), // N
    (hidio::HIDIOCommandID::HIDKeyboard, 0x10), // M
    (hidio::HIDIOCommandID::HIDKeyboard, 0x36), // Comma
    (hidio::HIDIOCommandID::HIDKeyboard, 0x37), // Period
    (hidio::HIDIOCommandID::HIDKeyboard, 0x38), // Slash
    (hidio::HIDIOCommandID::HIDKeyboard, 0xE5), // Right Shift
    (hidio::HIDIOCommandID::HIDKeyboard, 0x55), // Keypad Asterisk
    (hidio::HIDIOCommandID::HIDKeyboard, 0xE2), // Left Alt
    (hidio::HIDIOCommandID::HIDKeyboard, 0x2C), // Space
    (hidio::HIDIOCommandID::HIDKeyboard, 0x39), // Caps Lock
    (hidio::HIDIOCommandID::HIDKeyboard, 0x3A), // F1
    (hidio::HIDIOCommandID::HIDKeyboard, 0x3B), // F2
    (hidio::HIDIOCommandID::HIDKeyboard, 0x3C), // F3
    (hidio::HIDIOCommandID::HIDKeyboard, 0x3D), // F4
    (hidio::HIDIOCommandID::HIDKeyboard, 0x3E), // F5
    (hidio::HIDIOCommandID::HIDKeyboard, 0x3F), // F6
    (hidio::HIDIOCommandID::HIDKeyboard, 0x40), // F7
    (hidio::HIDIOCommandID::HIDKeyboard, 0x41), // F8
    (hidio::HIDIOCommandID::HIDKeyboard, 0x42), // F9
    (hidio::HIDIOCommandID::HIDKeyboard, 0x43), // F10
    (hidio::HIDIOCommandID::HIDKeyboard, 0x53), // Num Lock
    (hidio::HIDIOCommandID::HIDKeyboard, 0x47), // Scroll Lock
    (hidio::HIDIOCommandID::HIDKeyboard, 0x5F), // Keypad 7
    (hidio::HIDIOCommandID::HIDKeyboard, 0x60), // Keypad 8
    (hidio::HIDIOCommandID::HIDKeyboard, 0x61), // Keypad 9
    (hidio::HIDIOCommandID::HIDKeyboard, 0x56), // Keypad Minus
    (hidio::HIDIOCommandID::HIDKeyboard, 0x5C), // Keypad 4
    (hidio::HIDIOCommandID::HIDKeyboard, 0x5D), // Keypad 5
    (hidio::HIDIOCommandID::HIDKeyboard, 0x5E), // Keypad 6
    (hidio::HIDIOCommandID::HIDKeyboard, 0x57), // Keypad Plus
    (hidio::HIDIOCommandID::HIDKeyboard, 0x59), // Keypad 1
    (hidio::HIDIOCommandID::HIDKeyboard, 0x5A), // Keypad 2
    (hidio::HIDIOCommandID::HIDKeyboard, 0x5B), // Keypad 3
    (hidio::HIDIOCommandID::HIDKeyboard, 0x62), // Keypad 0
    (hidio::HIDIOCommandID::HIDKeyboard, 0x63), // Keypad Period
    (hidio::HIDIOCommandID::HIDKeyboard, 0x94), // LANG5 (Zenkakuhanku)
    (hidio::HIDIOCommandID::HIDKeyboard, 0x64), // ISO Slash
    (hidio::HIDIOCommandID::HIDKeyboard, 0x44), // F11
    (hidio::HIDIOCommandID::HIDKeyboard, 0x45), // F12
    (hidio::HIDIOCommandID::Unused, 0),         // TODO RO - 89
    (hidio::HIDIOCommandID::HIDKeyboard, 0x92), // LANG3 (Katakana)
    (hidio::HIDIOCommandID::HIDKeyboard, 0x93), // LANG4 (Hiragana)
    (hidio::HIDIOCommandID::HIDKeyboard, 0x8A), // International4 (Henkan)
    (hidio::HIDIOCommandID::HIDKeyboard, 0x88), // International2 (Katakana/Hiragana or Kana)
    (hidio::HIDIOCommandID::HIDKeyboard, 0x8B), // International5 (Muhenkan)
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KPJP Comma
    (hidio::HIDIOCommandID::HIDKeyboard, 0x58), // Keypad Enter
    (hidio::HIDIOCommandID::HIDKeyboard, 0xE4), // Right Control
    (hidio::HIDIOCommandID::HIDKeyboard, 0x54), // Keypad Slash
    (hidio::HIDIOCommandID::HIDKeyboard, 0x9A), // SysReq
    (hidio::HIDIOCommandID::HIDKeyboard, 0xE6), // Right Alt
    (hidio::HIDIOCommandID::Unused, 0),         // TODO Linefeed - 101
    (hidio::HIDIOCommandID::HIDKeyboard, 0x4A), // Home
    (hidio::HIDIOCommandID::HIDKeyboard, 0x52), // Up
    (hidio::HIDIOCommandID::HIDKeyboard, 0x4B), // Page Up
    (hidio::HIDIOCommandID::HIDKeyboard, 0x50), // Left
    (hidio::HIDIOCommandID::HIDKeyboard, 0x4F), // Right
    (hidio::HIDIOCommandID::HIDKeyboard, 0x4D), // End
    (hidio::HIDIOCommandID::HIDKeyboard, 0x51), // Down
    (hidio::HIDIOCommandID::HIDKeyboard, 0x4E), // Page Down
    (hidio::HIDIOCommandID::HIDKeyboard, 0x49), // Insert
    (hidio::HIDIOCommandID::HIDKeyboard, 0x4C), // Delete
    (hidio::HIDIOCommandID::Unused, 0),         // TODO Macro - 112
    (hidio::HIDIOCommandID::HIDKeyboard, 0x7F), // Mute
    (hidio::HIDIOCommandID::HIDKeyboard, 0x81), // Volume Down
    (hidio::HIDIOCommandID::HIDKeyboard, 0x80), // Volume Up
    (hidio::HIDIOCommandID::HIDConsumerCtrl, 0x030), // Power
    (hidio::HIDIOCommandID::HIDKeyboard, 0x67), // Keypad Equal
    (hidio::HIDIOCommandID::HIDKeyboard, 0xD7), // Keypad Plus Minus
    (hidio::HIDIOCommandID::HIDKeyboard, 0x48), // Pause
    (hidio::HIDIOCommandID::Unused, 0),         // TODO Scale - 120
    (hidio::HIDIOCommandID::HIDKeyboard, 0x85), // Keypad Comma
    (hidio::HIDIOCommandID::HIDKeyboard, 0x90), // LANG1 (Hangeul)
    (hidio::HIDIOCommandID::HIDKeyboard, 0x91), // LANG2 (Hanja)
    (hidio::HIDIOCommandID::HIDKeyboard, 0x89), // International3 (Yen)
    (hidio::HIDIOCommandID::HIDKeyboard, 0xE3), // Left GUI
    (hidio::HIDIOCommandID::HIDKeyboard, 0xE7), // Right GUI
    (hidio::HIDIOCommandID::Unused, 0),         // TODO Compose - 127
    (hidio::HIDIOCommandID::HIDKeyboard, 0x78), // Stop
    (hidio::HIDIOCommandID::HIDKeyboard, 0x79), // Again
    (hidio::HIDIOCommandID::Unused, 0),         // TODO Props - 130
    (hidio::HIDIOCommandID::HIDKeyboard, 0x7A), // Undo
    (hidio::HIDIOCommandID::Unused, 0),         // TODO Front - 132
    (hidio::HIDIOCommandID::HIDKeyboard, 0x7C), // Copy
    (hidio::HIDIOCommandID::HIDConsumerCtrl, 0x202), // Open
    (hidio::HIDIOCommandID::HIDKeyboard, 0x7D), // Paste
    (hidio::HIDIOCommandID::HIDKeyboard, 0x7E), // Find
    (hidio::HIDIOCommandID::HIDKeyboard, 0x7B), // Cut
    (hidio::HIDIOCommandID::HIDKeyboard, 0x75), // Help
    (hidio::HIDIOCommandID::HIDKeyboard, 0x76), // Menu
    (hidio::HIDIOCommandID::HIDConsumerCtrl, 0x192), // Calc
    (hidio::HIDIOCommandID::HIDSystemCtrl, 0xA2), // Setup
    (hidio::HIDIOCommandID::HIDSystemCtrl, 0x82), // Sleep
    (hidio::HIDIOCommandID::HIDSystemCtrl, 0x83), // Wakeup
    (hidio::HIDIOCommandID::Unused, 0),         // TODO File - 144
    (hidio::HIDIOCommandID::Unused, 0),         // TODO SendFile - 145
    (hidio::HIDIOCommandID::Unused, 0),         // TODO DeleteFile - 146
    (hidio::HIDIOCommandID::Unused, 0),         // TODO XFER - 147
    (hidio::HIDIOCommandID::Unused, 0),         // TODO PROG1 - 148
    (hidio::HIDIOCommandID::Unused, 0),         // TODO PROG2 - 149
    (hidio::HIDIOCommandID::Unused, 0),         // TODO WWW - 150
    (hidio::HIDIOCommandID::Unused, 0),         // TODO MSDOS - 151
    (hidio::HIDIOCommandID::Unused, 0),         // TODO COFFEE - 152
    (hidio::HIDIOCommandID::Unused, 0),         // TODO ROTATE DISPLAY - 153
    (hidio::HIDIOCommandID::Unused, 0),         // TODO CYCLE WINDOWS - 154
    (hidio::HIDIOCommandID::Unused, 0),         // TODO MAIL - 155
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BOOKMARKS - 156
    (hidio::HIDIOCommandID::Unused, 0),         // TODO COMPUTER - 157
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BACK - 158
    (hidio::HIDIOCommandID::Unused, 0),         // TODO FORWARD - 159
    (hidio::HIDIOCommandID::Unused, 0),         // TODO CLOSECD - 160
    (hidio::HIDIOCommandID::Unused, 0),         // TODO EJECTCD - 161
    (hidio::HIDIOCommandID::Unused, 0),         // TODO EJECTCLOSECD - 162
    (hidio::HIDIOCommandID::Unused, 0),         // TODO NEXTSONG - 163
    (hidio::HIDIOCommandID::Unused, 0),         // TODO PLAYPAUSE - 164
    (hidio::HIDIOCommandID::Unused, 0),         // TODO PREVIOUSSONG - 165
    (hidio::HIDIOCommandID::Unused, 0),         // TODO STOPCD - 166
    (hidio::HIDIOCommandID::Unused, 0),         // TODO RECORD - 167
    (hidio::HIDIOCommandID::Unused, 0),         // TODO REWIND - 168
    (hidio::HIDIOCommandID::Unused, 0),         // TODO PHONE - 169
    (hidio::HIDIOCommandID::Unused, 0),         // TODO ISO - 170
    (hidio::HIDIOCommandID::Unused, 0),         // TODO CONFIG - 171
    (hidio::HIDIOCommandID::Unused, 0),         // TODO HOMEPAGE - 172
    (hidio::HIDIOCommandID::Unused, 0),         // TODO REFRESH - 173
    (hidio::HIDIOCommandID::Unused, 0),         // TODO EXIT - 174
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_MOVE = 175,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_EDIT = 176,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_SCROLLUP = 177,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_SCROLLDOWN = 178,
    (hidio::HIDIOCommandID::HIDKeyboard, 0xB6), // Keypad Left Parenthesis
    (hidio::HIDIOCommandID::HIDKeyboard, 0xB7), // Keypad Right Parenthesis
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_NEW = 181,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_REDO = 182,
    (hidio::HIDIOCommandID::HIDKeyboard, 0x68), // F13
    (hidio::HIDIOCommandID::HIDKeyboard, 0x69), // F14
    (hidio::HIDIOCommandID::HIDKeyboard, 0x6A), // F15
    (hidio::HIDIOCommandID::HIDKeyboard, 0x6B), // F16
    (hidio::HIDIOCommandID::HIDKeyboard, 0x6C), // F17
    (hidio::HIDIOCommandID::HIDKeyboard, 0x6D), // F18
    (hidio::HIDIOCommandID::HIDKeyboard, 0x6E), // F19
    (hidio::HIDIOCommandID::HIDKeyboard, 0x6F), // F20
    (hidio::HIDIOCommandID::HIDKeyboard, 0x70), // F21
    (hidio::HIDIOCommandID::HIDKeyboard, 0x71), // F22
    (hidio::HIDIOCommandID::HIDKeyboard, 0x72), // F23
    (hidio::HIDIOCommandID::HIDKeyboard, 0x73), // F24
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_PLAYCD = 200,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_PAUSECD = 201,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_PROG3 = 202,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_PROG4 = 203,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_DASHBOARD = 204,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_SUSPEND = 205,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_CLOSE = 206,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_PLAY = 207,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_FASTFORWARD = 208,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_BASSBOOST = 209,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_PRINT = 210,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_HP = 211,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_CAMERA = 212,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_SOUND = 213,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_QUESTION = 214,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_EMAIL = 215,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_CHAT = 216,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_SEARCH = 217,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_CONNECT = 218,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_FINANCE = 219,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_SPORT = 220,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_SHOP = 221,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_ALTERASE = 222,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_CANCEL = 223,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_BRIGHTNESSDOWN = 224,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_BRIGHTNESSUP = 225,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_MEDIA = 226,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_SWITCHVIDEOMODE = 227,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_KBDILLUMTOGGLE = 228,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_KBDILLUMDOWN = 229,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_KBDILLUMUP = 230,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_SEND = 231,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_REPLY = 232,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_FORWARDMAIL = 233,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_SAVE = 234,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_DOCUMENTS = 235,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_BATTERY = 236,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_BLUETOOTH = 237,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_WLAN = 238,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_UWB = 239,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_UNKNOWN = 240,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_VIDEO_NEXT = 241,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_VIDEO_PREV = 242,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_BRIGHTNESS_CYCLE = 243,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_BRIGHTNESS_AUTO = 244,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_DISPLAY_OFF = 245,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_WWAN = 246,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_RFKILL = 247,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_MICMUTE = 248,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_OK = 352,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_SELECT = 353,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_GOTO = 354,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_CLEAR = 355,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_POWER2 = 356,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_OPTION = 357,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_INFO = 358,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_TIME = 359,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_VENDOR = 360,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_ARCHIVE = 361,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_PROGRAM = 362,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_CHANNEL = 363,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_FAVORITES = 364,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_EPG = 365,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_PVR = 366,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_MHP = 367,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_LANGUAGE = 368,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_TITLE = 369,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_SUBTITLE = 370,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_ANGLE = 371,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_FULL_SCREEN = 372,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_MODE = 373,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_KEYBOARD = 374,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_ASPECT_RATIO = 375,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_PC = 376,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_TV = 377,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_TV2 = 378,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_VCR = 379,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_VCR2 = 380,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_SAT = 381,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_SAT2 = 382,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_CD = 383,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_TAPE = 384,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_RADIO = 385,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_TUNER = 386,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_PLAYER = 387,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_TEXT = 388,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_DVD = 389,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_AUX = 390,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_MP3 = 391,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_AUDIO = 392,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_VIDEO = 393,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_DIRECTORY = 394,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_LIST = 395,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_MEMO = 396,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_CALENDAR = 397,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_RED = 398,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_GREEN = 399,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_YELLOW = 400,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_BLUE = 401,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_CHANNELUP = 402,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_CHANNELDOWN = 403,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_FIRST = 404,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_LAST = 405,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_AB = 406,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_NEXT = 407,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_RESTART = 408,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_SLOW = 409,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_SHUFFLE = 410,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_BREAK = 411,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_PREVIOUS = 412,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_DIGITS = 413,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_TEEN = 414,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_TWEN = 415,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_VIDEOPHONE = 416,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_GAMES = 417,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_ZOOMIN = 418,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_ZOOMOUT = 419,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_ZOOMRESET = 420,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_WORDPROCESSOR = 421,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_EDITOR = 422,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_SPREADSHEET = 423,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_GRAPHICSEDITOR = 424,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_PRESENTATION = 425,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_DATABASE = 426,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_NEWS = 427,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_VOICEMAIL = 428,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_ADDRESSBOOK = 429,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_MESSENGER = 430,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_DISPLAYTOGGLE = 431,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_SPELLCHECK = 432,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_LOGOFF = 433,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_DOLLAR = 434,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_EURO = 435,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_FRAMEBACK = 436,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_FRAMEFORWARD = 437,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_CONTEXT_MENU = 438,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_MEDIA_REPEAT = 439,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_10CHANNELSUP = 440,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_10CHANNELSDOWN = 441,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_IMAGES = 442,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_DEL_EOL = 448,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_DEL_EOS = 449,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_INS_LINE = 450,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_DEL_LINE = 451,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_FN = 464,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_FN_ESC = 465,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_FN_F1 = 466,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_FN_F2 = 467,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_FN_F3 = 468,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_FN_F4 = 469,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_FN_F5 = 470,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_FN_F6 = 471,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_FN_F7 = 472,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_FN_F8 = 473,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_FN_F9 = 474,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_FN_F10 = 475,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_FN_F11 = 476,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_FN_F12 = 477,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_FN_1 = 478,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_FN_2 = 479,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_FN_D = 480,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_FN_E = 481,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_FN_F = 482,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_FN_S = 483,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_FN_B = 484,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_BRL_DOT1 = 497,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_BRL_DOT2 = 498,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_BRL_DOT3 = 499,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_BRL_DOT4 = 500,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_BRL_DOT5 = 501,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_BRL_DOT6 = 502,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_BRL_DOT7 = 503,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_BRL_DOT8 = 504,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_BRL_DOT9 = 505,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_BRL_DOT10 = 506,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_NUMERIC_0 = 512,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_NUMERIC_1 = 513,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_NUMERIC_2 = 514,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_NUMERIC_3 = 515,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_NUMERIC_4 = 516,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_NUMERIC_5 = 517,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_NUMERIC_6 = 518,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_NUMERIC_7 = 519,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_NUMERIC_8 = 520,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_NUMERIC_9 = 521,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_NUMERIC_STAR = 522,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_NUMERIC_POUND = 523,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_NUMERIC_A = 524,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_NUMERIC_B = 525,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_NUMERIC_C = 526,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_NUMERIC_D = 527,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_CAMERA_FOCUS = 528,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_WPS_BUTTON = 529,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_TOUCHPAD_TOGGLE = 530,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_TOUCHPAD_ON = 531,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_TOUCHPAD_OFF = 532,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_CAMERA_ZOOMIN = 533,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_CAMERA_ZOOMOUT = 534,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_CAMERA_UP = 535,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_CAMERA_DOWN = 536,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_CAMERA_LEFT = 537,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_CAMERA_RIGHT = 538,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_ATTENDANT_ON = 539,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_ATTENDANT_OFF = 540,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_ATTENDANT_TOGGLE = 541,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_LIGHTS_TOGGLE = 542,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_ALS_TOGGLE = 560,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_ROTATE_LOCK_TOGGLE = 561,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_BUTTONCONFIG = 576,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_TASKMANAGER = 577,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_JOURNAL = 578,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_CONTROLPANEL = 579,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_APPSELECT = 580,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_SCREENSAVER = 581,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_VOICECOMMAND = 582,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_ASSISTANT = 583,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_KBD_LAYOUT_NEXT = 584,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_BRIGHTNESS_MIN = 592,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_BRIGHTNESS_MAX = 593,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_KBDINPUTASSIST_PREV = 608,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_KBDINPUTASSIST_NEXT = 609,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_KBDINPUTASSIST_PREVGROUP = 610,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_KBDINPUTASSIST_NEXTGROUP = 611,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_KBDINPUTASSIST_ACCEPT = 612,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_KBDINPUTASSIST_CANCEL = 613,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_RIGHT_UP = 614,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_RIGHT_DOWN = 615,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_LEFT_UP = 616,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_LEFT_DOWN = 617,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_ROOT_MENU = 618,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_MEDIA_TOP_MENU = 619,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_NUMERIC_11 = 620,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_NUMERIC_12 = 621,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_AUDIO_DESC = 622,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_3D_MODE = 623,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_NEXT_FAVORITE = 624,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_STOP_RECORD = 625,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_PAUSE_RECORD = 626,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_VOD = 627,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_UNMUTE = 628,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_FASTREVERSE = 629,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_SLOWREVERSE = 630,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_DATA = 631,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_ONSCREEN_KEYBOARD = 632,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO KEY_MAX = 767,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_0 = 256,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_1 = 257,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_2 = 258,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_3 = 259,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_4 = 260,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_5 = 261,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_6 = 262,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_7 = 263,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_8 = 264,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_9 = 265,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_LEFT = 272,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_RIGHT = 273,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_MIDDLE = 274,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_SIDE = 275,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_EXTRA = 276,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_FORWARD = 277,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_BACK = 278,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_TASK = 279,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_TRIGGER = 288,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_THUMB = 289,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_THUMB2 = 290,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_TOP = 291,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_TOP2 = 292,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_PINKIE = 293,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_BASE = 294,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_BASE2 = 295,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_BASE3 = 296,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_BASE4 = 297,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_BASE5 = 298,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_BASE6 = 299,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_DEAD = 303,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_SOUTH = 304,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_EAST = 305,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_C = 306,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_NORTH = 307,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_WEST = 308,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_Z = 309,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_TL = 310,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_TR = 311,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_TL2 = 312,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_TR2 = 313,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_SELECT = 314,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_START = 315,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_MODE = 316,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_THUMBL = 317,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_THUMBR = 318,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_TOOL_PEN = 320,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_TOOL_RUBBER = 321,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_TOOL_BRUSH = 322,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_TOOL_PENCIL = 323,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_TOOL_AIRBRUSH = 324,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_TOOL_FINGER = 325,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_TOOL_MOUSE = 326,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_TOOL_LENS = 327,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_TOOL_QUINTTAP = 328,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_STYLUS3 = 329,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_TOUCH = 330,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_STYLUS = 331,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_STYLUS2 = 332,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_TOOL_DOUBLETAP = 333,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_TOOL_TRIPLETAP = 334,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_TOOL_QUADTAP = 335,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_GEAR_DOWN = 336,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_GEAR_UP = 337,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_DPAD_UP = 544,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_DPAD_DOWN = 545,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_DPAD_LEFT = 546,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_DPAD_RIGHT = 547,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_TRIGGER_HAPPY1 = 704,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_TRIGGER_HAPPY2 = 705,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_TRIGGER_HAPPY3 = 706,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_TRIGGER_HAPPY4 = 707,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_TRIGGER_HAPPY5 = 708,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_TRIGGER_HAPPY6 = 709,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_TRIGGER_HAPPY7 = 710,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_TRIGGER_HAPPY8 = 711,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_TRIGGER_HAPPY9 = 712,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_TRIGGER_HAPPY10 = 713,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_TRIGGER_HAPPY11 = 714,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_TRIGGER_HAPPY12 = 715,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_TRIGGER_HAPPY13 = 716,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_TRIGGER_HAPPY14 = 717,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_TRIGGER_HAPPY15 = 718,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_TRIGGER_HAPPY16 = 719,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_TRIGGER_HAPPY17 = 720,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_TRIGGER_HAPPY18 = 721,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_TRIGGER_HAPPY19 = 722,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_TRIGGER_HAPPY20 = 723,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_TRIGGER_HAPPY21 = 724,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_TRIGGER_HAPPY22 = 725,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_TRIGGER_HAPPY23 = 726,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_TRIGGER_HAPPY24 = 727,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_TRIGGER_HAPPY25 = 728,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_TRIGGER_HAPPY26 = 729,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_TRIGGER_HAPPY27 = 730,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_TRIGGER_HAPPY28 = 731,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_TRIGGER_HAPPY29 = 732,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_TRIGGER_HAPPY30 = 733,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_TRIGGER_HAPPY31 = 734,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_TRIGGER_HAPPY32 = 735,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_TRIGGER_HAPPY33 = 736,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_TRIGGER_HAPPY34 = 737,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_TRIGGER_HAPPY35 = 738,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_TRIGGER_HAPPY36 = 739,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_TRIGGER_HAPPY37 = 740,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_TRIGGER_HAPPY38 = 741,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_TRIGGER_HAPPY39 = 742,
    (hidio::HIDIOCommandID::Unused, 0),         // TODO BTN_TRIGGER_HAPPY40 = 743,

                                                // TODO TODO Figure out these keys on evdev
                                                /*
                                                0x32 // Number
                                                0x46 // Print Screen
                                                0x65 // App
                                                0x66 // Keyboard Status
                                                0x74 // Exec
                                                0x77 // Select
                                                0x82 // Locking Caps Lock
                                                0x83 // Locking Num Lock
                                                0x84 // Locking Scroll Lock
                                                0x86 // Keypad Equal AS400
                                                0x87 // International1
                                                0x8C // International6
                                                0x8D // International7
                                                0x8E // International8
                                                0x8F // International9
                                                0x95 // LANG6
                                                0x96 // LANG7
                                                0x97 // LANG8
                                                0x98 // LANG9
                                                0x99 // Alternate Erase
                                                0x9B // Cancel
                                                0x9C // Clear
                                                0x9D // Prior
                                                0x9E // Return
                                                0x9F // Separator
                                                0xA0 // Out
                                                0xA1 // Oper
                                                0xA2 // Clear Again
                                                0xA3 // CrSel Props
                                                0xA4 // ExSel

                                                0xB0 // Keypad 00
                                                0xB1 // Keypad 000
                                                0xB2 // 1000 Separator
                                                0xB3 // Decimal Separator
                                                0xB4 // Currency Unit
                                                0xB5 // Currency SubUnit
                                                0xB8 // Keypad Left Brace
                                                0xB9 // Keypad Right Brace
                                                0xBA // Keypad Tab
                                                0xBB // Keypad Backspace
                                                0xBC // Keypad A
                                                0xBD // Keypad B
                                                0xBE // Keypad C
                                                0xBF // Keypad D
                                                0xC0 // Keypad E
                                                0xC1 // Keypad F
                                                0xC2 // Keypad XOR
                                                0xC3 // Keypad Chevron
                                                0xC4 // Keypad Percent
                                                0xC5 // Keypad Less Than
                                                0xC6 // Keypad Greater Than
                                                0xC7 // Keypad BITAND
                                                0xC8 // Keypad AND
                                                0xC9 // Keypad BITOR
                                                0xCA // Keypad OR
                                                0xCB // Keypad Colon
                                                0xCC // Keypad Hash
                                                0xCD // Keypad Space
                                                0xCE // Keypad At
                                                0xCF // Keypad Exclamation
                                                0xD0 // Keypad Memory Store
                                                0xD1 // Keypad Memory Recall
                                                0xD2 // Keypad Memory Clear
                                                0xD3 // Keypad Memory Add
                                                0xD4 // Keypad Memory Subtract
                                                0xD5 // Keypad Memory Multiply
                                                0xD6 // Keypad Memory Divide
                                                0xD8 // Keypad Clear
                                                0xD9 // Keypad Clear Entry
                                                0xDA // Keypad Binary
                                                0xDB // Keypad Octal
                                                0xDC // Keypad Decimal
                                                0xDD // Keypad Hexidecimal
                                                */
];

/// Convert evdev codes into hid codes
fn evdev2basehid(
    code: evdev_rs::enums::EventCode,
) -> std::io::Result<(hidio::HIDIOCommandID, u16)> {
    use evdev_rs::enums::EventCode;
    match code.clone() {
        EventCode::EV_KEY(key) => {
            // Do an ev code to hid code lookup
            // Will error if no lookup is available
            let lookup = EVDEV2HIDKEY[key as usize];
            if lookup.0 == hidio::HIDIOCommandID::Unused {
                Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("No key hid code lookup for ev code: {}", code),
                ))
            } else {
                Ok(lookup)
            }
        }
        _ => Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("No hid code lookup for ev code: {}", code),
        )),
    }
}

/// Device state container for evdev devices
pub struct EvdevDevice {
    mailbox: mailbox::Mailbox,
    uid: u64,
    endpoint: Endpoint,
    fd_path: String,
}

impl EvdevDevice {
    pub fn new(mailbox: mailbox::Mailbox, fd_path: String) -> std::io::Result<EvdevDevice> {
        // We query evdev here for information, but we don't grab the input until running process()
        // Initialize new evdev handle
        let mut device = match evdev_rs::Device::new() {
            Some(device) => device,
            None => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Could not create evdev device",
                ));
            }
        };

        // Apply file descriptor to evdev handle
        let file = std::fs::File::open(fd_path.clone())?;
        device.set_fd(file)?;

        // Determine type of device
        let devtype = device_type(&device, fd_path.clone())?;

        // Assign uid to newly created device (need path location for uniqueness)
        let mut evdev_info = EvdevInfo::new(device);
        let uid = mailbox
            .clone()
            .assign_uid(evdev_info.key(), fd_path.clone())
            .unwrap();

        // Setup Endpoint
        let mut endpoint = Endpoint::new(devtype, uid);
        endpoint.set_evdev_params(evdev_info);

        // Register node
        mailbox.clone().register_node(endpoint.clone());

        Ok(EvdevDevice {
            mailbox,
            uid,
            endpoint,
            fd_path,
        })
    }

    // Process evdev events
    pub fn process(&mut self) -> std::io::Result<()> {
        let fd_path = self.fd_path.clone();

        // Initialize new evdev handle
        let mut device = match evdev_rs::Device::new() {
            Some(device) => device,
            None => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Could not create evdev device",
                ));
            }
        };

        // Apply file descriptor to evdev handle
        let file = std::fs::File::open(fd_path)?;
        device.set_fd(file)?;

        // Take all event information (block events from other processes)
        device.grab(evdev_rs::GrabMode::Grab).unwrap();

        // Queue up evdev events to send
        // Each event is received individually, but we want all events that come from an
        // instance in time (in order to emulate how hid devices send devices; as well as how
        // HIDIO packets send state)
        let mut event_queue: Vec<evdev_rs::InputEvent> = vec![];
        let mut event_queue_command = hidio::HIDIOCommandID::HIDKeyboard; // Default to a keyboard message
        let mut drop_until_next_syn_report = false;

        let mut event: std::io::Result<(evdev_rs::ReadStatus, evdev_rs::InputEvent)>;
        // Continuously scan for new events
        // This loop will block at next_event()
        loop {
            // TODO Implement ppoll (or similar) like on udev to handle timeout (to get the latency as low
            // as possible without pinning the cpu)
            // Currently we are just blocking and using a tokio blocking thread (also low latency)
            // However it's difficult to end this cleanly.
            event = device.next_event(evdev_rs::ReadFlag::NORMAL | evdev_rs::ReadFlag::BLOCKING);
            if event.is_ok() {
                let mut result = event.ok().unwrap();
                // TODO send event message through mailbox
                debug!(
                    "{:?} {:?} {}",
                    &result.1.event_type, &result.1.event_code, &result.1.value
                );

                match result.0 {
                    evdev_rs::ReadStatus::Sync => {
                        // Dropped packet (this shouldn't happen)
                        // We should warn about it though
                        warn!("Dropped evdev event! - Attempting to resync...");
                        while result.0 == evdev_rs::ReadStatus::Sync {
                            warn!(
                                "Dropped: {:?} {:?} {}",
                                &result.1.event_type, &result.1.event_code, &result.1.value
                            );
                            event = device.next_event(evdev_rs::ReadFlag::SYNC);
                            if event.is_ok() {
                                result = event.ok().unwrap();
                            } else {
                                return Ok(());
                            }
                        }
                        warn!("Resyncing successful.");
                    }
                    evdev_rs::ReadStatus::Success => {
                        match &result.1.event_code {
                            // Check if we've received an EV_SYN(SYN_REPORT) which indicates the event
                            // queue should be flushed
                            evdev_rs::enums::EventCode::EV_SYN(
                                evdev_rs::enums::EV_SYN::SYN_REPORT,
                            ) => {
                                if drop_until_next_syn_report {
                                    // Drop any queued events
                                    event_queue = vec![];
                                    drop_until_next_syn_report = false;
                                } else {
                                    // - Send enqueued events -
                                    // Generate HIDIO packet data
                                    let data = match event_queue_command {
                                        hidio::HIDIOCommandID::HIDKeyboard => {
                                            // Convert evdev codes into base hid codes
                                            let mut data = vec![];
                                            for event in event_queue.clone() {
                                                let code = event.event_code;
                                                match evdev2basehid(code) {
                                                    Ok(code) => {
                                                        // TODO Handle SystemCtrl and ConsumerCtrl
                                                        if code.0
                                                            == hidio::HIDIOCommandID::HIDKeyboard
                                                        {
                                                            // Handle press/release
                                                            if event.value == 1 {
                                                                data.push(code.1 as u8);
                                                            } else {
                                                                data.retain(|&x| x != code.1 as u8);
                                                            }
                                                        } else {
                                                            // Skip unhandled mapped codes
                                                            warn!("Skipping: {:?}", code);
                                                            continue;
                                                        }
                                                    }
                                                    Err(msg) => {
                                                        // Skip code if there is an error
                                                        warn!("Err: {:?}", msg);
                                                        continue;
                                                    }
                                                }
                                            }
                                            data
                                        }
                                        // TODO Currently ignoring other send events
                                        _ => {
                                            debug!("Ignoring send: {:?}", event_queue);
                                            continue;
                                        }
                                    };

                                    // Encode the message as a HIDIO packet
                                    self.mailbox.send_command(
                                        mailbox::Address::DeviceHid { uid: self.uid },
                                        mailbox::Address::All,
                                        event_queue_command,
                                        data,
                                    );
                                }
                                continue;
                            }
                            // Check if we've received an EV_SYN(SYN_DROPPED) which indicates all
                            // events until *after* the *next* EV_SYN(SYN_REPORT) should be dropped
                            evdev_rs::enums::EventCode::EV_SYN(
                                evdev_rs::enums::EV_SYN::SYN_DROPPED,
                            ) => {
                                drop_until_next_syn_report = true;
                                continue;
                            }
                            _ => {}
                        }

                        // Select the type of HIDIO Packet being sent based off of the device type
                        let command = match self.endpoint.type_() {
                            common_capnp::NodeType::HidKeyboard => {
                                // Filter for keyboard events
                                if !&result.1.is_type(&evdev_rs::enums::EventType::EV_KEY) {
                                    continue;
                                }

                                hidio::HIDIOCommandID::HIDKeyboard
                            }
                            common_capnp::NodeType::HidMouse => {
                                // Filter for mouse events
                                // TODO
                                // TODO We may need to handle more complicated mouse packets
                                hidio::HIDIOCommandID::HIDMouse
                            }
                            common_capnp::NodeType::HidJoystick => {
                                // Filter for joystick events
                                // TODO
                                // TODO We may need to handle more complicated joystick packets
                                hidio::HIDIOCommandID::HIDJoystick
                            }
                            _ => {
                                panic!(
                                    "Unknown type for EvdevDevice endpoint node: {}",
                                    self.endpoint.type_()
                                );
                            }
                        };

                        // Enqueue event
                        event_queue.push(result.1);
                    }
                }
            } else {
                // Disconnection event, shutdown processing loop
                // This object should be deallocated as well
                let err = event.err().unwrap();
                match err.raw_os_error() {
                    Some(libc::EAGAIN) => continue,
                    _ => {
                        info!("Disconnection event {}", device_name(device));
                        return Ok(());
                    }
                }
            }

            // TODO Check if there are more events, if yes, keep trying to enqueue
        }
        Ok(())
    }
}

impl Drop for EvdevDevice {
    fn drop(&mut self) {
        // Unregister node
        self.mailbox.unregister_node(self.uid);
    }
}

/// Build a unique device name string
fn device_name(device: evdev_rs::Device) -> String {
    let mut string = format!(
        "[{:04x}:{:04x}-{:?}] {} {} {}",
        device.vendor_id(),
        device.product_id(),
        evdev_rs::enums::int_to_bus_type(device.bustype() as u32),
        device.name().unwrap_or(""),
        device.phys().unwrap_or(""),
        device.uniq().unwrap_or(""),
    );
    string
}

// From evdev types, determine what type of hid-io device this is
// Scanned in order of Keyboard, Mouse then Joystick
// Keyboard
// - Has one of two homing keys
// Mouse
// - Has the left mouse button
// Joystick
// - Has a trigger button
fn device_type(
    device: &evdev_rs::Device,
    fd_path: String,
) -> std::io::Result<common_capnp::NodeType> {
    use evdev_rs::enums::*;

    Ok(
        if device.has(&EventCode::EV_KEY(EV_KEY::KEY_F))
            || device.has(&EventCode::EV_KEY(EV_KEY::KEY_J))
        {
            common_capnp::NodeType::HidKeyboard
        } else if device.has(&EventCode::EV_KEY(EV_KEY::BTN_LEFT)) {
            common_capnp::NodeType::HidMouse
        } else if device.has(&EventCode::EV_KEY(EV_KEY::BTN_TRIGGER)) {
            common_capnp::NodeType::HidJoystick
        } else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("{} is not a keyboard, mouse or joystick", fd_path),
            ));
        },
    )
}

/// evdev processing
///
/// TODO
/// udev to wait on new evdev devices
/// udev to scan for already attached devices
/// Allocate uid per unique device
/// Have list of evdev devices to query
/// Handle removal and re-insertion with same uid
/// Use async to wait for evdev events (block on next event, using spawn_blocking)
/// Send mailbox message with necessary info (API will handle re-routing message)

/// hidapi processing
///
/// This thread periodically refreshes the USB device list to see if a new device needs to be attached
/// The thread also handles reading/writing from connected interfaces
///
/// XXX (HaaTa) hidapi is not thread-safe on all platforms, so don't try to create a thread per device
/*
async fn processing(mut mailbox: mailbox::Mailbox) {
    info!("Spawning hidapi spawning thread...");

    // Initialize HID interface
    let mut api = ::hidapi::HidApi::new().expect("HID API object creation failed");

    let mut devices: Vec<HIDIOController> = vec![];

    let mut last_scan = Instant::now();
    let mut enumerate = true;

    // Loop infinitely, the watcher only exits if the daemon is quit
    loop {
        while enumerate {
            if !RUNNING.load(Ordering::SeqCst) {
                return;
            }

            // Refresh devices list
            api.refresh_devices().unwrap();

            // Iterate over found USB interfaces and select usable ones
            debug!("Scanning for devices");
            for device_info in api.device_list() {
                let device_str = format!(
                    "Device: {:#?}\n    {} R:{}",
                    device_info.path(),
                    device_name(device_info),
                    device_info.release_number()
                );
                debug!("{}", device_str);

                // Use usage page and usage for matching HID-IO compatible device
                if !match_device(device_info) {
                    continue;
                }

                // Build set of HID info to make unique comparisons
                let mut info = HIDAPIInfo::new(device_info);

                // Determine if id can be reused
                // Criteria
                // 1. Must match (even if field isn't valid)
                //    vid, pid, usage page, usage, manufacturer, product, serial, interface
                // 2. Must not currently be in use (generally, use path to differentiate)
                let key = info.build_hidapi_key();
                let uid = match mailbox.get_uid(key.clone(), format!("{:#?}", device_info.path())) {
                    Some(0) => {
                        // Device has already been registered
                        continue;
                    }
                    Some(uid) => uid,
                    None => {
                        // Get last created id and increment
                        (*mailbox.last_uid.write().unwrap()) += 1;
                        let uid = *mailbox.last_uid.read().unwrap();

                        // Add id to lookup
                        mailbox.add_uid(key, uid);
                        uid
                    }
                };

                // Check to see if already connected
                if devices.iter().any(|dev| dev.uid == uid) {
                    continue;
                }

                // Add device
                info!("Connecting to uid:{} {}", uid, device_str);

                // If serial number is a MAC address, this is a bluetooth device
                lazy_static! {
                    static ref RE: Regex =
                        Regex::new(r"([0-9a-fA-F][0-9a-fA-F]:){5}([0-9a-fA-F][0-9a-fA-F])")
                            .unwrap();
                }
                let is_ble = RE.is_match(match device_info.serial_number() {
                    Some(s) => s,
                    _ => "",
                });

                // Create node
                let mut node = Endpoint::new(
                    if is_ble {
                        NodeType::BleKeyboard
                    } else {
                        NodeType::UsbKeyboard
                    },
                    uid,
                );
                node.set_hidapi_params(info);

                // Connect to device
                debug!("Attempt to open {:#?}", node);
                match api.open_path(device_info.path()) {
                    Ok(device) => {
                        println!("Connected to {}", node);
                        let device = HIDAPIDevice::new(device);
                        let mut device =
                            HIDIOEndpoint::new(Box::new(device), USB_FULLSPEED_PACKET_SIZE as u32);

                        if let Err(e) = device.send_sync() {
                            // Could not open device (likely removed, or in use)
                            warn!("Processing - {}", e);
                            continue;
                        }

                        // Setup device controller (handles communication and protocol conversion
                        // for the HIDIO device)
                        let master = HIDIOController::new(mailbox.clone(), uid, device);
                        devices.push(master);

                        // Add device to node list
                        mailbox.nodes.write().unwrap().push(node);
                    }
                    Err(e) => {
                        // Could not open device (likely removed, or in use)
                        warn!("Processing - {}", e);
                        continue;
                    }
                };
            }

            // Update scan time
            last_scan = Instant::now();

            if !devices.is_empty() {
                debug!("Enumeration finished");
                enumerate = false;
            } else {
                // Sleep so we don't starve the CPU
                // TODO (HaaTa) - There should be a better way to watch the ports, but still be responsive
                // XXX - Rewrite hidapi with rust and include async
                tokio::time::delay_for(std::time::Duration::from_millis(ENUMERATE_DELAY)).await;
            }
        }

        loop {
            if !RUNNING.load(Ordering::SeqCst) {
                return;
            }

            if devices.is_empty() {
                info!("No connected devices. Forcing scan");
                enumerate = true;
                break;
            }

            // TODO (HaaTa): Make command-line argument/config option
            if last_scan.elapsed().as_millis() >= 1000 {
                enumerate = true;
                break;
            }

            // Process devices
            let mut removed_devices = vec![];
            let mut io_events: usize = 0;
            devices = devices
                .drain_filter(|dev| {
                    // Check if disconnected
                    let ret = dev.process();
                    let result = ret.is_ok();
                    if ret.is_err() {
                        removed_devices.push(dev.uid);
                        info!("{} disconnected. No longer polling it", dev.uid);
                    } else {
                        // Record io events (used to schedule sleeps)
                        io_events += ret.ok().unwrap();
                    }
                    result
                })
                .collect::<Vec<_>>();

            // Modify nodes list to remove any uids that were disconnected
            // uids are unique across both api and devices, so this is always safe to do
            if !removed_devices.is_empty() {
                let new_nodes = mailbox
                    .nodes
                    .read()
                    .unwrap()
                    .clone()
                    .drain_filter(|node| !removed_devices.contains(&node.uid()))
                    .collect::<Vec<_>>();
                *mailbox.nodes.write().unwrap() = new_nodes;
            }

            // If there was any IO, on any of the devices, do not sleep, only sleep when all devices are idle
            if io_events == 0 {
                tokio::time::delay_for(std::time::Duration::from_millis(POLL_DELAY)).await;
            }
        }
    }
}
*/

/// evdev initialization
///
/// Sets up processing threads for udev and evdev.
pub async fn initialize(mailbox: mailbox::Mailbox) {
    info!("Initializing device/evdev...");

    // Spawn watcher thread (tokio)
    /*
    let local = tokio::task::LocalSet::new();
    local.run_until(processing(mailbox)).await;
    */
}

/// Finds an input event device handle using udev
pub fn udev_find_input_event_device(
    vid: u16,
    pid: u16,
    subsystem: String,
    uniq: String,
    timeout: std::time::Duration,
) -> Result<udev::Device, std::io::Error> {
    match vhid::uhid::udev_find_device(vid, pid, subsystem, uniq, timeout) {
        Ok(device) => {
            let mut enumerator = udev::Enumerator::new().unwrap();
            enumerator.match_parent(&device).unwrap();
            enumerator.match_subsystem("input").unwrap();
            let mut evdevice: EvdevDevice;
            let mut found = false;

            // Validate parameters
            for device in enumerator.scan_devices().unwrap() {
                let fd_path = format!(
                    "/dev/input/{}",
                    device.sysname().to_str().unwrap().to_string()
                );
                if fd_path.contains("event") {
                    return Ok(device);
                }
            }
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Cound not find input event device...",
            ))
        }
        Err(err) => Err(err),
    }
}

#[test]
fn uhid_evdev_keyboard_test() {
    setup_logging_lite();
    // Create uhid keyboard interface
    let name = "evdev-keyboard-nkro-test".to_string();
    let mailbox = mailbox::Mailbox::new();

    // Generate a unique key (to handle parallel tests)
    let uniq = nanoid::simple();

    // Instantiate hid device
    let mut keyboard = vhid::uhid::KeyboardNKRO::new(
        mailbox.clone(),
        name.clone(),
        "".to_string(),
        uniq.clone(),
        uhid_virt::Bus::USB,
        vhid::IC_VID as u32,
        vhid::IC_PID_KEYBOARD as u32,
        0,
        0,
    )
    .unwrap();

    // Make sure device is there (will poll for a while just in case uhid/kernel is slow)
    let device = match udev_find_input_event_device(
        vhid::IC_VID as u16,
        vhid::IC_PID_KEYBOARD as u16,
        "input".to_string(),
        uniq,
        std::time::Duration::new(10, 0),
    ) {
        Ok(device) => device,
        Err(err) => {
            panic!("Could not find udev device... {}", err);
        }
    };

    // Find evdev mapping to uhid device
    while !device.is_initialized() {} // Wait for udev to finish setting up device
    let fd_path = format!(
        "/dev/input/{}",
        device.sysname().to_str().unwrap().to_string()
    );

    // Now that both uhid and evdev nodes are setup we can attempt to send some keypresses to
    // validate that evdev is working correctly
    // However, before we can send any keypresses, a mailbox receiver is setup to watch for the incoming
    // messages
    let mut receiver = mailbox.sender.subscribe(); // Subscribe to mailbox messages

    let mut rt = tokio::runtime::Runtime::new().unwrap();

    // Start listening for mailbox messages
    rt.spawn(async move {
        // These are the expected messages
        // Due to how evdev works, it's possible that at least one additional empty packet will be
        // sent. Just ignore any extra packets.
        let expected_msgs = vec![vec![4], vec![4, 5], vec![5], vec![]];
        let mut msg_pos = 0;

        loop {
            match receiver.recv().await {
                Ok(msg) => {
                    // Keep listening for extra messages after completing the verification
                    if msg_pos + 1 == expected_msgs.len() {
                        continue;
                    }

                    // Verify the incoming keypresses
                    if msg.data.data == expected_msgs[msg_pos] {
                        msg_pos += 1;
                    } else {
                        assert!(msg.data.data == vec![], "Unexpected message: {:?}", msg);
                    }
                }
                Err(tokio::sync::broadcast::RecvError::Closed) => {
                    assert!(false, "Mailbox has been closed unexpectedly!");
                }
                Err(tokio::sync::broadcast::RecvError::Lagged(skipped)) => {
                    assert!(
                        false,
                        "Mailbox has received too many messages, lagging by: {}",
                        skipped
                    );
                }
            };
        }
    });

    // Start listening for evdev events
    rt.spawn(async move {
        tokio::task::spawn_blocking(move || {
            EvdevDevice::new(mailbox.clone(), fd_path)
                .unwrap()
                .process()
                .unwrap();
        });
    });

    //std::thread::sleep(std::time::Duration::from_millis(100));

    rt.block_on(async {
        // Make sure everything is initialized and monitoring
        tokio::time::delay_for(std::time::Duration::from_millis(100)).await;

        // Send A;A,B;B key using uhid device
        // TODO integrate layouts-rs from HID-IO (to have symbolic testing inputs)
        keyboard.send(vec![4]);
        keyboard.send(vec![4, 5]);
        keyboard.send(vec![5]);
        keyboard.send(vec![0]);

        // Give some time for the events to propagate
        tokio::time::delay_for(std::time::Duration::from_millis(100)).await;
    });

    // Force the runtime to shutdown
    rt.shutdown_timeout(std::time::Duration::from_millis(100));
}
