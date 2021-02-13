#![cfg(all(feature = "dev-capture", target_os = "linux"))]
/* Copyright (C) 2020-2021 by Jacob Alexander
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

// ----- Crates -----

use crate::api::common_capnp;
use crate::api::Endpoint;
use crate::api::EvdevInfo;
use crate::mailbox;
use crate::module::vhid;
use hid_io_protocol::*;

// TODO This should be converted to use hid-io/layouts (may need a rust package to handle
// conversion)
const EVDEV2HIDKEY: [(HidIoCommandId, u16); 548] = [
    (HidIoCommandId::HidKeyboard, 0x00),      // 0   Reserved
    (HidIoCommandId::HidKeyboard, 0x29),      // 1   Esc
    (HidIoCommandId::HidKeyboard, 0x1E),      // 2   1
    (HidIoCommandId::HidKeyboard, 0x1F),      // 3   2
    (HidIoCommandId::HidKeyboard, 0x20),      // 4   3
    (HidIoCommandId::HidKeyboard, 0x21),      // 5   4
    (HidIoCommandId::HidKeyboard, 0x22),      // 6   5
    (HidIoCommandId::HidKeyboard, 0x23),      // 7   6
    (HidIoCommandId::HidKeyboard, 0x24),      // 8   7
    (HidIoCommandId::HidKeyboard, 0x25),      // 9   8
    (HidIoCommandId::HidKeyboard, 0x26),      // 10  9
    (HidIoCommandId::HidKeyboard, 0x27),      // 11  0
    (HidIoCommandId::HidKeyboard, 0x2D),      // 12  Minus
    (HidIoCommandId::HidKeyboard, 0x2E),      // 13  Equal
    (HidIoCommandId::HidKeyboard, 0x2A),      // 14  Backspace
    (HidIoCommandId::HidKeyboard, 0x2B),      // 15  Tab
    (HidIoCommandId::HidKeyboard, 0x14),      // 16  Q
    (HidIoCommandId::HidKeyboard, 0x1A),      // 17  W
    (HidIoCommandId::HidKeyboard, 0x08),      // 18  E
    (HidIoCommandId::HidKeyboard, 0x15),      // 19  R
    (HidIoCommandId::HidKeyboard, 0x17),      // 20  T
    (HidIoCommandId::HidKeyboard, 0x1C),      // 21  Y
    (HidIoCommandId::HidKeyboard, 0x18),      // 22  U
    (HidIoCommandId::HidKeyboard, 0x0C),      // 23  I
    (HidIoCommandId::HidKeyboard, 0x12),      // 24  O
    (HidIoCommandId::HidKeyboard, 0x13),      // 25  P
    (HidIoCommandId::HidKeyboard, 0x2F),      // 26  Left Bracket
    (HidIoCommandId::HidKeyboard, 0x30),      // 27  Right Bracket
    (HidIoCommandId::HidKeyboard, 0x28),      // 28  Enter
    (HidIoCommandId::HidKeyboard, 0xE0),      // 29  Left Control
    (HidIoCommandId::HidKeyboard, 0x04),      // 30  A
    (HidIoCommandId::HidKeyboard, 0x16),      // 31  S
    (HidIoCommandId::HidKeyboard, 0x07),      // 32  D
    (HidIoCommandId::HidKeyboard, 0x09),      // 33  F
    (HidIoCommandId::HidKeyboard, 0x0A),      // 34  G
    (HidIoCommandId::HidKeyboard, 0x0B),      // 35  H
    (HidIoCommandId::HidKeyboard, 0x0D),      // 36  J
    (HidIoCommandId::HidKeyboard, 0x0E),      // 37  K
    (HidIoCommandId::HidKeyboard, 0x0F),      // 38  L
    (HidIoCommandId::HidKeyboard, 0x33),      // 39  Semicolon
    (HidIoCommandId::HidKeyboard, 0x34),      // 40  Quote
    (HidIoCommandId::HidKeyboard, 0x35),      // 41  Backtick
    (HidIoCommandId::HidKeyboard, 0xE1),      // 42  Left Shift
    (HidIoCommandId::HidKeyboard, 0x31),      // 43  Backslash
    (HidIoCommandId::HidKeyboard, 0x1D),      // 44  Z
    (HidIoCommandId::HidKeyboard, 0x1B),      // 45  X
    (HidIoCommandId::HidKeyboard, 0x06),      // 46 C
    (HidIoCommandId::HidKeyboard, 0x19),      // 47  V
    (HidIoCommandId::HidKeyboard, 0x05),      // 48  B
    (HidIoCommandId::HidKeyboard, 0x11),      // 49  N
    (HidIoCommandId::HidKeyboard, 0x10),      // 50  M
    (HidIoCommandId::HidKeyboard, 0x36),      // 51  Comma
    (HidIoCommandId::HidKeyboard, 0x37),      // 52  Period
    (HidIoCommandId::HidKeyboard, 0x38),      // 53  Slash
    (HidIoCommandId::HidKeyboard, 0xE5),      // 54  Right Shift
    (HidIoCommandId::HidKeyboard, 0x55),      // 55  Keypad Asterisk
    (HidIoCommandId::HidKeyboard, 0xE2),      // 56  Left Alt
    (HidIoCommandId::HidKeyboard, 0x2C),      // 57  Space
    (HidIoCommandId::HidKeyboard, 0x39),      // 58  Caps Lock
    (HidIoCommandId::HidKeyboard, 0x3A),      // 59  F1
    (HidIoCommandId::HidKeyboard, 0x3B),      // 60  F2
    (HidIoCommandId::HidKeyboard, 0x3C),      // 61  F3
    (HidIoCommandId::HidKeyboard, 0x3D),      // 62  F4
    (HidIoCommandId::HidKeyboard, 0x3E),      // 63  F5
    (HidIoCommandId::HidKeyboard, 0x3F),      // 64  F6
    (HidIoCommandId::HidKeyboard, 0x40),      // 65  F7
    (HidIoCommandId::HidKeyboard, 0x41),      // 66  F8
    (HidIoCommandId::HidKeyboard, 0x42),      // 67  F9
    (HidIoCommandId::HidKeyboard, 0x43),      // 68  F10
    (HidIoCommandId::HidKeyboard, 0x53),      // 69  Num Lock
    (HidIoCommandId::HidKeyboard, 0x47),      // 70  Scroll Lock
    (HidIoCommandId::HidKeyboard, 0x5F),      // 71  Keypad 7
    (HidIoCommandId::HidKeyboard, 0x60),      // 72  Keypad 8
    (HidIoCommandId::HidKeyboard, 0x61),      // 73  Keypad 9
    (HidIoCommandId::HidKeyboard, 0x56),      // 74  Keypad Minus
    (HidIoCommandId::HidKeyboard, 0x5C),      // 75  Keypad 4
    (HidIoCommandId::HidKeyboard, 0x5D),      // 76  Keypad 5
    (HidIoCommandId::HidKeyboard, 0x5E),      // 77  Keypad 6
    (HidIoCommandId::HidKeyboard, 0x57),      // 78  Keypad Plus
    (HidIoCommandId::HidKeyboard, 0x59),      // 79  Keypad 1
    (HidIoCommandId::HidKeyboard, 0x5A),      // 80  Keypad 2
    (HidIoCommandId::HidKeyboard, 0x5B),      // 81  Keypad 3
    (HidIoCommandId::HidKeyboard, 0x62),      // 82  Keypad 0
    (HidIoCommandId::HidKeyboard, 0x63),      // 83  Keypad Period
    (HidIoCommandId::Unused, 0),              // TODO ??? - 84
    (HidIoCommandId::HidKeyboard, 0x94),      // 85  LANG5 (Zenkakuhanku)
    (HidIoCommandId::HidKeyboard, 0x64),      // 86  ISO Slash
    (HidIoCommandId::HidKeyboard, 0x44),      // 87  F11
    (HidIoCommandId::HidKeyboard, 0x45),      // 88  F12
    (HidIoCommandId::Unused, 0),              // TODO RO - 89
    (HidIoCommandId::HidKeyboard, 0x92),      // 90  LANG3 (Katakana)
    (HidIoCommandId::HidKeyboard, 0x93),      // 91  LANG4 (Hiragana)
    (HidIoCommandId::HidKeyboard, 0x8A),      // 92  International4 (Henkan)
    (HidIoCommandId::HidKeyboard, 0x88),      // 93  International2 (Katakana/Hiragana or Kana)
    (HidIoCommandId::HidKeyboard, 0x8B),      // 94  International5 (Muhenkan)
    (HidIoCommandId::Unused, 0),              // TODO KPJP Comma 95
    (HidIoCommandId::HidKeyboard, 0x58),      // 96  Keypad Enter
    (HidIoCommandId::HidKeyboard, 0xE4),      // 97  Right Control
    (HidIoCommandId::HidKeyboard, 0x54),      // 98  Keypad Slash
    (HidIoCommandId::HidKeyboard, 0x9A),      // 99  SysReq
    (HidIoCommandId::HidKeyboard, 0xE6),      // 100 Right Alt
    (HidIoCommandId::Unused, 0),              // TODO Linefeed - 101
    (HidIoCommandId::HidKeyboard, 0x4A),      // 102 Home
    (HidIoCommandId::HidKeyboard, 0x52),      // 103 Up
    (HidIoCommandId::HidKeyboard, 0x4B),      // 104 Page Up
    (HidIoCommandId::HidKeyboard, 0x50),      // 105 Left
    (HidIoCommandId::HidKeyboard, 0x4F),      // 106 Right
    (HidIoCommandId::HidKeyboard, 0x4D),      // 107 End
    (HidIoCommandId::HidKeyboard, 0x51),      // 108 Down
    (HidIoCommandId::HidKeyboard, 0x4E),      // 109 Page Down
    (HidIoCommandId::HidKeyboard, 0x49),      // 110 Insert
    (HidIoCommandId::HidKeyboard, 0x4C),      // 111 Delete
    (HidIoCommandId::Unused, 0),              // TODO Macro - 112
    (HidIoCommandId::HidKeyboard, 0x7F),      // 113 Mute
    (HidIoCommandId::HidKeyboard, 0x81),      // 114 Volume Down
    (HidIoCommandId::HidKeyboard, 0x80),      // 115 Volume Up
    (HidIoCommandId::HidConsumerCtrl, 0x030), // 116 Power
    (HidIoCommandId::HidKeyboard, 0x67),      // 117 Keypad Equal
    (HidIoCommandId::HidKeyboard, 0xD7),      // 118 Keypad Plus Minus
    (HidIoCommandId::HidKeyboard, 0x48),      // 119 Pause
    (HidIoCommandId::Unused, 0),              // TODO Scale - 120
    (HidIoCommandId::HidKeyboard, 0x85),      // 121 Keypad Comma
    (HidIoCommandId::HidKeyboard, 0x90),      // 122 LANG1 (Hangeul)
    (HidIoCommandId::HidKeyboard, 0x91),      // 123 LANG2 (Hanja)
    (HidIoCommandId::HidKeyboard, 0x89),      // 124 International3 (Yen)
    (HidIoCommandId::HidKeyboard, 0xE3),      // 125 Left GUI
    (HidIoCommandId::HidKeyboard, 0xE7),      // 126 Right GUI
    (HidIoCommandId::Unused, 0),              // TODO Compose - 127
    (HidIoCommandId::HidKeyboard, 0x78),      // 128 Stop
    (HidIoCommandId::HidKeyboard, 0x79),      // 129 Again
    (HidIoCommandId::Unused, 0),              // TODO Props - 130
    (HidIoCommandId::HidKeyboard, 0x7A),      // 131 Undo
    (HidIoCommandId::Unused, 0),              // TODO Front - 132
    (HidIoCommandId::HidKeyboard, 0x7C),      // 133 Copy
    (HidIoCommandId::HidConsumerCtrl, 0x202), // 134 Open
    (HidIoCommandId::HidKeyboard, 0x7D),      // 135 Paste
    (HidIoCommandId::HidKeyboard, 0x7E),      // 136 Find
    (HidIoCommandId::HidKeyboard, 0x7B),      // 137 Cut
    (HidIoCommandId::HidKeyboard, 0x75),      // 138 Help
    (HidIoCommandId::HidKeyboard, 0x76),      // 139 Menu
    (HidIoCommandId::HidConsumerCtrl, 0x192), // 140 Calc
    (HidIoCommandId::HidSystemCtrl, 0xA2),    // 141 Setup
    (HidIoCommandId::HidSystemCtrl, 0x82),    // 142 Sleep
    (HidIoCommandId::HidSystemCtrl, 0x83),    // 143 Wakeup
    (HidIoCommandId::Unused, 0),              // TODO File - 144
    (HidIoCommandId::Unused, 0),              // TODO SendFile - 145
    (HidIoCommandId::Unused, 0),              // TODO DeleteFile - 146
    (HidIoCommandId::Unused, 0),              // TODO XFER - 147
    (HidIoCommandId::Unused, 0),              // TODO PROG1 - 148
    (HidIoCommandId::Unused, 0),              // TODO PROG2 - 149
    (HidIoCommandId::Unused, 0),              // TODO WWW - 150
    (HidIoCommandId::Unused, 0),              // TODO MSDOS - 151
    (HidIoCommandId::Unused, 0),              // TODO COFFEE - 152
    (HidIoCommandId::Unused, 0),              // TODO ROTATE DISPLAY - 153
    (HidIoCommandId::Unused, 0),              // TODO CYCLE WINDOWS - 154
    (HidIoCommandId::Unused, 0),              // TODO MAIL - 155
    (HidIoCommandId::Unused, 0),              // TODO BOOKMARKS - 156
    (HidIoCommandId::Unused, 0),              // TODO COMPUTER - 157
    (HidIoCommandId::Unused, 0),              // TODO BACK - 158
    (HidIoCommandId::Unused, 0),              // TODO FORWARD - 159
    (HidIoCommandId::Unused, 0),              // TODO CLOSECD - 160
    (HidIoCommandId::Unused, 0),              // TODO EJECTCD - 161
    (HidIoCommandId::Unused, 0),              // TODO EJECTCLOSECD - 162
    (HidIoCommandId::Unused, 0),              // TODO NEXTSONG - 163
    (HidIoCommandId::Unused, 0),              // TODO PLAYPAUSE - 164
    (HidIoCommandId::Unused, 0),              // TODO PREVIOUSSONG - 165
    (HidIoCommandId::Unused, 0),              // TODO STOPCD - 166
    (HidIoCommandId::Unused, 0),              // TODO RECORD - 167
    (HidIoCommandId::Unused, 0),              // TODO REWIND - 168
    (HidIoCommandId::Unused, 0),              // TODO PHONE - 169
    (HidIoCommandId::Unused, 0),              // TODO ISO - 170
    (HidIoCommandId::Unused, 0),              // TODO CONFIG - 171
    (HidIoCommandId::Unused, 0),              // TODO HOMEPAGE - 172
    (HidIoCommandId::Unused, 0),              // TODO REFRESH - 173
    (HidIoCommandId::Unused, 0),              // TODO EXIT - 174
    (HidIoCommandId::Unused, 0),              // TODO KEY_MOVE = 175,
    (HidIoCommandId::Unused, 0),              // TODO KEY_EDIT = 176,
    (HidIoCommandId::Unused, 0),              // TODO KEY_SCROLLUP = 177,
    (HidIoCommandId::Unused, 0),              // TODO KEY_SCROLLDOWN = 178,
    (HidIoCommandId::HidKeyboard, 0xB6),      // Keypad Left Parenthesis
    (HidIoCommandId::HidKeyboard, 0xB7),      // Keypad Right Parenthesis
    (HidIoCommandId::Unused, 0),              // TODO KEY_NEW = 181,
    (HidIoCommandId::Unused, 0),              // TODO KEY_REDO = 182,
    (HidIoCommandId::HidKeyboard, 0x68),      // F13
    (HidIoCommandId::HidKeyboard, 0x69),      // F14
    (HidIoCommandId::HidKeyboard, 0x6A),      // F15
    (HidIoCommandId::HidKeyboard, 0x6B),      // F16
    (HidIoCommandId::HidKeyboard, 0x6C),      // F17
    (HidIoCommandId::HidKeyboard, 0x6D),      // F18
    (HidIoCommandId::HidKeyboard, 0x6E),      // F19
    (HidIoCommandId::HidKeyboard, 0x6F),      // F20
    (HidIoCommandId::HidKeyboard, 0x70),      // F21
    (HidIoCommandId::HidKeyboard, 0x71),      // F22
    (HidIoCommandId::HidKeyboard, 0x72),      // F23
    (HidIoCommandId::HidKeyboard, 0x73),      // F24
    (HidIoCommandId::Unused, 0),              // TODO KEY_PLAYCD = 200,
    (HidIoCommandId::Unused, 0),              // TODO KEY_PAUSECD = 201,
    (HidIoCommandId::Unused, 0),              // TODO KEY_PROG3 = 202,
    (HidIoCommandId::Unused, 0),              // TODO KEY_PROG4 = 203,
    (HidIoCommandId::Unused, 0),              // TODO KEY_DASHBOARD = 204,
    (HidIoCommandId::Unused, 0),              // TODO KEY_SUSPEND = 205,
    (HidIoCommandId::Unused, 0),              // TODO KEY_CLOSE = 206,
    (HidIoCommandId::Unused, 0),              // TODO KEY_PLAY = 207,
    (HidIoCommandId::Unused, 0),              // TODO KEY_FASTFORWARD = 208,
    (HidIoCommandId::Unused, 0),              // TODO KEY_BASSBOOST = 209,
    (HidIoCommandId::Unused, 0),              // TODO KEY_PRINT = 210,
    (HidIoCommandId::Unused, 0),              // TODO KEY_HP = 211,
    (HidIoCommandId::Unused, 0),              // TODO KEY_CAMERA = 212,
    (HidIoCommandId::Unused, 0),              // TODO KEY_SOUND = 213,
    (HidIoCommandId::Unused, 0),              // TODO KEY_QUESTION = 214,
    (HidIoCommandId::Unused, 0),              // TODO KEY_EMAIL = 215,
    (HidIoCommandId::Unused, 0),              // TODO KEY_CHAT = 216,
    (HidIoCommandId::Unused, 0),              // TODO KEY_SEARCH = 217,
    (HidIoCommandId::Unused, 0),              // TODO KEY_CONNECT = 218,
    (HidIoCommandId::Unused, 0),              // TODO KEY_FINANCE = 219,
    (HidIoCommandId::Unused, 0),              // TODO KEY_SPORT = 220,
    (HidIoCommandId::Unused, 0),              // TODO KEY_SHOP = 221,
    (HidIoCommandId::Unused, 0),              // TODO KEY_ALTERASE = 222,
    (HidIoCommandId::Unused, 0),              // TODO KEY_CANCEL = 223,
    (HidIoCommandId::Unused, 0),              // TODO KEY_BRIGHTNESSDOWN = 224,
    (HidIoCommandId::Unused, 0),              // TODO KEY_BRIGHTNESSUP = 225,
    (HidIoCommandId::Unused, 0),              // TODO KEY_MEDIA = 226,
    (HidIoCommandId::Unused, 0),              // TODO KEY_SWITCHVIDEOMODE = 227,
    (HidIoCommandId::Unused, 0),              // TODO KEY_KBDILLUMTOGGLE = 228,
    (HidIoCommandId::Unused, 0),              // TODO KEY_KBDILLUMDOWN = 229,
    (HidIoCommandId::Unused, 0),              // TODO KEY_KBDILLUMUP = 230,
    (HidIoCommandId::Unused, 0),              // TODO KEY_SEND = 231,
    (HidIoCommandId::Unused, 0),              // TODO KEY_REPLY = 232,
    (HidIoCommandId::Unused, 0),              // TODO KEY_FORWARDMAIL = 233,
    (HidIoCommandId::Unused, 0),              // TODO KEY_SAVE = 234,
    (HidIoCommandId::Unused, 0),              // TODO KEY_DOCUMENTS = 235,
    (HidIoCommandId::Unused, 0),              // TODO KEY_BATTERY = 236,
    (HidIoCommandId::Unused, 0),              // TODO KEY_BLUETOOTH = 237,
    (HidIoCommandId::Unused, 0),              // TODO KEY_WLAN = 238,
    (HidIoCommandId::Unused, 0),              // TODO KEY_UWB = 239,
    (HidIoCommandId::Unused, 0),              // TODO KEY_UNKNOWN = 240,
    (HidIoCommandId::Unused, 0),              // TODO KEY_VIDEO_NEXT = 241,
    (HidIoCommandId::Unused, 0),              // TODO KEY_VIDEO_PREV = 242,
    (HidIoCommandId::Unused, 0),              // TODO KEY_BRIGHTNESS_CYCLE = 243,
    (HidIoCommandId::Unused, 0),              // TODO KEY_BRIGHTNESS_AUTO = 244,
    (HidIoCommandId::Unused, 0),              // TODO KEY_DISPLAY_OFF = 245,
    (HidIoCommandId::Unused, 0),              // TODO KEY_WWAN = 246,
    (HidIoCommandId::Unused, 0),              // TODO KEY_RFKILL = 247,
    (HidIoCommandId::Unused, 0),              // TODO KEY_MICMUTE = 248,
    (HidIoCommandId::Unused, 0),              // TODO KEY_OK = 352,
    (HidIoCommandId::Unused, 0),              // TODO KEY_SELECT = 353,
    (HidIoCommandId::Unused, 0),              // TODO KEY_GOTO = 354,
    (HidIoCommandId::Unused, 0),              // TODO KEY_CLEAR = 355,
    (HidIoCommandId::Unused, 0),              // TODO KEY_POWER2 = 356,
    (HidIoCommandId::Unused, 0),              // TODO KEY_OPTION = 357,
    (HidIoCommandId::Unused, 0),              // TODO KEY_INFO = 358,
    (HidIoCommandId::Unused, 0),              // TODO KEY_TIME = 359,
    (HidIoCommandId::Unused, 0),              // TODO KEY_VENDOR = 360,
    (HidIoCommandId::Unused, 0),              // TODO KEY_ARCHIVE = 361,
    (HidIoCommandId::Unused, 0),              // TODO KEY_PROGRAM = 362,
    (HidIoCommandId::Unused, 0),              // TODO KEY_CHANNEL = 363,
    (HidIoCommandId::Unused, 0),              // TODO KEY_FAVORITES = 364,
    (HidIoCommandId::Unused, 0),              // TODO KEY_EPG = 365,
    (HidIoCommandId::Unused, 0),              // TODO KEY_PVR = 366,
    (HidIoCommandId::Unused, 0),              // TODO KEY_MHP = 367,
    (HidIoCommandId::Unused, 0),              // TODO KEY_LANGUAGE = 368,
    (HidIoCommandId::Unused, 0),              // TODO KEY_TITLE = 369,
    (HidIoCommandId::Unused, 0),              // TODO KEY_SUBTITLE = 370,
    (HidIoCommandId::Unused, 0),              // TODO KEY_ANGLE = 371,
    (HidIoCommandId::Unused, 0),              // TODO KEY_FULL_SCREEN = 372,
    (HidIoCommandId::Unused, 0),              // TODO KEY_MODE = 373,
    (HidIoCommandId::Unused, 0),              // TODO KEY_KEYBOARD = 374,
    (HidIoCommandId::Unused, 0),              // TODO KEY_ASPECT_RATIO = 375,
    (HidIoCommandId::Unused, 0),              // TODO KEY_PC = 376,
    (HidIoCommandId::Unused, 0),              // TODO KEY_TV = 377,
    (HidIoCommandId::Unused, 0),              // TODO KEY_TV2 = 378,
    (HidIoCommandId::Unused, 0),              // TODO KEY_VCR = 379,
    (HidIoCommandId::Unused, 0),              // TODO KEY_VCR2 = 380,
    (HidIoCommandId::Unused, 0),              // TODO KEY_SAT = 381,
    (HidIoCommandId::Unused, 0),              // TODO KEY_SAT2 = 382,
    (HidIoCommandId::Unused, 0),              // TODO KEY_CD = 383,
    (HidIoCommandId::Unused, 0),              // TODO KEY_TAPE = 384,
    (HidIoCommandId::Unused, 0),              // TODO KEY_RADIO = 385,
    (HidIoCommandId::Unused, 0),              // TODO KEY_TUNER = 386,
    (HidIoCommandId::Unused, 0),              // TODO KEY_PLAYER = 387,
    (HidIoCommandId::Unused, 0),              // TODO KEY_TEXT = 388,
    (HidIoCommandId::Unused, 0),              // TODO KEY_DVD = 389,
    (HidIoCommandId::Unused, 0),              // TODO KEY_AUX = 390,
    (HidIoCommandId::Unused, 0),              // TODO KEY_MP3 = 391,
    (HidIoCommandId::Unused, 0),              // TODO KEY_AUDIO = 392,
    (HidIoCommandId::Unused, 0),              // TODO KEY_VIDEO = 393,
    (HidIoCommandId::Unused, 0),              // TODO KEY_DIRECTORY = 394,
    (HidIoCommandId::Unused, 0),              // TODO KEY_LIST = 395,
    (HidIoCommandId::Unused, 0),              // TODO KEY_MEMO = 396,
    (HidIoCommandId::Unused, 0),              // TODO KEY_CALENDAR = 397,
    (HidIoCommandId::Unused, 0),              // TODO KEY_RED = 398,
    (HidIoCommandId::Unused, 0),              // TODO KEY_GREEN = 399,
    (HidIoCommandId::Unused, 0),              // TODO KEY_YELLOW = 400,
    (HidIoCommandId::Unused, 0),              // TODO KEY_BLUE = 401,
    (HidIoCommandId::Unused, 0),              // TODO KEY_CHANNELUP = 402,
    (HidIoCommandId::Unused, 0),              // TODO KEY_CHANNELDOWN = 403,
    (HidIoCommandId::Unused, 0),              // TODO KEY_FIRST = 404,
    (HidIoCommandId::Unused, 0),              // TODO KEY_LAST = 405,
    (HidIoCommandId::Unused, 0),              // TODO KEY_AB = 406,
    (HidIoCommandId::Unused, 0),              // TODO KEY_NEXT = 407,
    (HidIoCommandId::Unused, 0),              // TODO KEY_RESTART = 408,
    (HidIoCommandId::Unused, 0),              // TODO KEY_SLOW = 409,
    (HidIoCommandId::Unused, 0),              // TODO KEY_SHUFFLE = 410,
    (HidIoCommandId::Unused, 0),              // TODO KEY_BREAK = 411,
    (HidIoCommandId::Unused, 0),              // TODO KEY_PREVIOUS = 412,
    (HidIoCommandId::Unused, 0),              // TODO KEY_DIGITS = 413,
    (HidIoCommandId::Unused, 0),              // TODO KEY_TEEN = 414,
    (HidIoCommandId::Unused, 0),              // TODO KEY_TWEN = 415,
    (HidIoCommandId::Unused, 0),              // TODO KEY_VIDEOPHONE = 416,
    (HidIoCommandId::Unused, 0),              // TODO KEY_GAMES = 417,
    (HidIoCommandId::Unused, 0),              // TODO KEY_ZOOMIN = 418,
    (HidIoCommandId::Unused, 0),              // TODO KEY_ZOOMOUT = 419,
    (HidIoCommandId::Unused, 0),              // TODO KEY_ZOOMRESET = 420,
    (HidIoCommandId::Unused, 0),              // TODO KEY_WORDPROCESSOR = 421,
    (HidIoCommandId::Unused, 0),              // TODO KEY_EDITOR = 422,
    (HidIoCommandId::Unused, 0),              // TODO KEY_SPREADSHEET = 423,
    (HidIoCommandId::Unused, 0),              // TODO KEY_GRAPHICSEDITOR = 424,
    (HidIoCommandId::Unused, 0),              // TODO KEY_PRESENTATION = 425,
    (HidIoCommandId::Unused, 0),              // TODO KEY_DATABASE = 426,
    (HidIoCommandId::Unused, 0),              // TODO KEY_NEWS = 427,
    (HidIoCommandId::Unused, 0),              // TODO KEY_VOICEMAIL = 428,
    (HidIoCommandId::Unused, 0),              // TODO KEY_ADDRESSBOOK = 429,
    (HidIoCommandId::Unused, 0),              // TODO KEY_MESSENGER = 430,
    (HidIoCommandId::Unused, 0),              // TODO KEY_DISPLAYTOGGLE = 431,
    (HidIoCommandId::Unused, 0),              // TODO KEY_SPELLCHECK = 432,
    (HidIoCommandId::Unused, 0),              // TODO KEY_LOGOFF = 433,
    (HidIoCommandId::Unused, 0),              // TODO KEY_DOLLAR = 434,
    (HidIoCommandId::Unused, 0),              // TODO KEY_EURO = 435,
    (HidIoCommandId::Unused, 0),              // TODO KEY_FRAMEBACK = 436,
    (HidIoCommandId::Unused, 0),              // TODO KEY_FRAMEFORWARD = 437,
    (HidIoCommandId::Unused, 0),              // TODO KEY_CONTEXT_MENU = 438,
    (HidIoCommandId::Unused, 0),              // TODO KEY_MEDIA_REPEAT = 439,
    (HidIoCommandId::Unused, 0),              // TODO KEY_10CHANNELSUP = 440,
    (HidIoCommandId::Unused, 0),              // TODO KEY_10CHANNELSDOWN = 441,
    (HidIoCommandId::Unused, 0),              // TODO KEY_IMAGES = 442,
    (HidIoCommandId::Unused, 0),              // TODO KEY_DEL_EOL = 448,
    (HidIoCommandId::Unused, 0),              // TODO KEY_DEL_EOS = 449,
    (HidIoCommandId::Unused, 0),              // TODO KEY_INS_LINE = 450,
    (HidIoCommandId::Unused, 0),              // TODO KEY_DEL_LINE = 451,
    (HidIoCommandId::Unused, 0),              // TODO KEY_FN = 464,
    (HidIoCommandId::Unused, 0),              // TODO KEY_FN_ESC = 465,
    (HidIoCommandId::Unused, 0),              // TODO KEY_FN_F1 = 466,
    (HidIoCommandId::Unused, 0),              // TODO KEY_FN_F2 = 467,
    (HidIoCommandId::Unused, 0),              // TODO KEY_FN_F3 = 468,
    (HidIoCommandId::Unused, 0),              // TODO KEY_FN_F4 = 469,
    (HidIoCommandId::Unused, 0),              // TODO KEY_FN_F5 = 470,
    (HidIoCommandId::Unused, 0),              // TODO KEY_FN_F6 = 471,
    (HidIoCommandId::Unused, 0),              // TODO KEY_FN_F7 = 472,
    (HidIoCommandId::Unused, 0),              // TODO KEY_FN_F8 = 473,
    (HidIoCommandId::Unused, 0),              // TODO KEY_FN_F9 = 474,
    (HidIoCommandId::Unused, 0),              // TODO KEY_FN_F10 = 475,
    (HidIoCommandId::Unused, 0),              // TODO KEY_FN_F11 = 476,
    (HidIoCommandId::Unused, 0),              // TODO KEY_FN_F12 = 477,
    (HidIoCommandId::Unused, 0),              // TODO KEY_FN_1 = 478,
    (HidIoCommandId::Unused, 0),              // TODO KEY_FN_2 = 479,
    (HidIoCommandId::Unused, 0),              // TODO KEY_FN_D = 480,
    (HidIoCommandId::Unused, 0),              // TODO KEY_FN_E = 481,
    (HidIoCommandId::Unused, 0),              // TODO KEY_FN_F = 482,
    (HidIoCommandId::Unused, 0),              // TODO KEY_FN_S = 483,
    (HidIoCommandId::Unused, 0),              // TODO KEY_FN_B = 484,
    (HidIoCommandId::Unused, 0),              // TODO KEY_BRL_DOT1 = 497,
    (HidIoCommandId::Unused, 0),              // TODO KEY_BRL_DOT2 = 498,
    (HidIoCommandId::Unused, 0),              // TODO KEY_BRL_DOT3 = 499,
    (HidIoCommandId::Unused, 0),              // TODO KEY_BRL_DOT4 = 500,
    (HidIoCommandId::Unused, 0),              // TODO KEY_BRL_DOT5 = 501,
    (HidIoCommandId::Unused, 0),              // TODO KEY_BRL_DOT6 = 502,
    (HidIoCommandId::Unused, 0),              // TODO KEY_BRL_DOT7 = 503,
    (HidIoCommandId::Unused, 0),              // TODO KEY_BRL_DOT8 = 504,
    (HidIoCommandId::Unused, 0),              // TODO KEY_BRL_DOT9 = 505,
    (HidIoCommandId::Unused, 0),              // TODO KEY_BRL_DOT10 = 506,
    (HidIoCommandId::Unused, 0),              // TODO KEY_NUMERIC_0 = 512,
    (HidIoCommandId::Unused, 0),              // TODO KEY_NUMERIC_1 = 513,
    (HidIoCommandId::Unused, 0),              // TODO KEY_NUMERIC_2 = 514,
    (HidIoCommandId::Unused, 0),              // TODO KEY_NUMERIC_3 = 515,
    (HidIoCommandId::Unused, 0),              // TODO KEY_NUMERIC_4 = 516,
    (HidIoCommandId::Unused, 0),              // TODO KEY_NUMERIC_5 = 517,
    (HidIoCommandId::Unused, 0),              // TODO KEY_NUMERIC_6 = 518,
    (HidIoCommandId::Unused, 0),              // TODO KEY_NUMERIC_7 = 519,
    (HidIoCommandId::Unused, 0),              // TODO KEY_NUMERIC_8 = 520,
    (HidIoCommandId::Unused, 0),              // TODO KEY_NUMERIC_9 = 521,
    (HidIoCommandId::Unused, 0),              // TODO KEY_NUMERIC_STAR = 522,
    (HidIoCommandId::Unused, 0),              // TODO KEY_NUMERIC_POUND = 523,
    (HidIoCommandId::Unused, 0),              // TODO KEY_NUMERIC_A = 524,
    (HidIoCommandId::Unused, 0),              // TODO KEY_NUMERIC_B = 525,
    (HidIoCommandId::Unused, 0),              // TODO KEY_NUMERIC_C = 526,
    (HidIoCommandId::Unused, 0),              // TODO KEY_NUMERIC_D = 527,
    (HidIoCommandId::Unused, 0),              // TODO KEY_CAMERA_FOCUS = 528,
    (HidIoCommandId::Unused, 0),              // TODO KEY_WPS_BUTTON = 529,
    (HidIoCommandId::Unused, 0),              // TODO KEY_TOUCHPAD_TOGGLE = 530,
    (HidIoCommandId::Unused, 0),              // TODO KEY_TOUCHPAD_ON = 531,
    (HidIoCommandId::Unused, 0),              // TODO KEY_TOUCHPAD_OFF = 532,
    (HidIoCommandId::Unused, 0),              // TODO KEY_CAMERA_ZOOMIN = 533,
    (HidIoCommandId::Unused, 0),              // TODO KEY_CAMERA_ZOOMOUT = 534,
    (HidIoCommandId::Unused, 0),              // TODO KEY_CAMERA_UP = 535,
    (HidIoCommandId::Unused, 0),              // TODO KEY_CAMERA_DOWN = 536,
    (HidIoCommandId::Unused, 0),              // TODO KEY_CAMERA_LEFT = 537,
    (HidIoCommandId::Unused, 0),              // TODO KEY_CAMERA_RIGHT = 538,
    (HidIoCommandId::Unused, 0),              // TODO KEY_ATTENDANT_ON = 539,
    (HidIoCommandId::Unused, 0),              // TODO KEY_ATTENDANT_OFF = 540,
    (HidIoCommandId::Unused, 0),              // TODO KEY_ATTENDANT_TOGGLE = 541,
    (HidIoCommandId::Unused, 0),              // TODO KEY_LIGHTS_TOGGLE = 542,
    (HidIoCommandId::Unused, 0),              // TODO KEY_ALS_TOGGLE = 560,
    (HidIoCommandId::Unused, 0),              // TODO KEY_ROTATE_LOCK_TOGGLE = 561,
    (HidIoCommandId::Unused, 0),              // TODO KEY_BUTTONCONFIG = 576,
    (HidIoCommandId::Unused, 0),              // TODO KEY_TASKMANAGER = 577,
    (HidIoCommandId::Unused, 0),              // TODO KEY_JOURNAL = 578,
    (HidIoCommandId::Unused, 0),              // TODO KEY_CONTROLPANEL = 579,
    (HidIoCommandId::Unused, 0),              // TODO KEY_APPSELECT = 580,
    (HidIoCommandId::Unused, 0),              // TODO KEY_SCREENSAVER = 581,
    (HidIoCommandId::Unused, 0),              // TODO KEY_VOICECOMMAND = 582,
    (HidIoCommandId::Unused, 0),              // TODO KEY_ASSISTANT = 583,
    (HidIoCommandId::Unused, 0),              // TODO KEY_KBD_LAYOUT_NEXT = 584,
    (HidIoCommandId::Unused, 0),              // TODO KEY_BRIGHTNESS_MIN = 592,
    (HidIoCommandId::Unused, 0),              // TODO KEY_BRIGHTNESS_MAX = 593,
    (HidIoCommandId::Unused, 0),              // TODO KEY_KBDINPUTASSIST_PREV = 608,
    (HidIoCommandId::Unused, 0),              // TODO KEY_KBDINPUTASSIST_NEXT = 609,
    (HidIoCommandId::Unused, 0),              // TODO KEY_KBDINPUTASSIST_PREVGROUP = 610,
    (HidIoCommandId::Unused, 0),              // TODO KEY_KBDINPUTASSIST_NEXTGROUP = 611,
    (HidIoCommandId::Unused, 0),              // TODO KEY_KBDINPUTASSIST_ACCEPT = 612,
    (HidIoCommandId::Unused, 0),              // TODO KEY_KBDINPUTASSIST_CANCEL = 613,
    (HidIoCommandId::Unused, 0),              // TODO KEY_RIGHT_UP = 614,
    (HidIoCommandId::Unused, 0),              // TODO KEY_RIGHT_DOWN = 615,
    (HidIoCommandId::Unused, 0),              // TODO KEY_LEFT_UP = 616,
    (HidIoCommandId::Unused, 0),              // TODO KEY_LEFT_DOWN = 617,
    (HidIoCommandId::Unused, 0),              // TODO KEY_ROOT_MENU = 618,
    (HidIoCommandId::Unused, 0),              // TODO KEY_MEDIA_TOP_MENU = 619,
    (HidIoCommandId::Unused, 0),              // TODO KEY_NUMERIC_11 = 620,
    (HidIoCommandId::Unused, 0),              // TODO KEY_NUMERIC_12 = 621,
    (HidIoCommandId::Unused, 0),              // TODO KEY_AUDIO_DESC = 622,
    (HidIoCommandId::Unused, 0),              // TODO KEY_3D_MODE = 623,
    (HidIoCommandId::Unused, 0),              // TODO KEY_NEXT_FAVORITE = 624,
    (HidIoCommandId::Unused, 0),              // TODO KEY_STOP_RECORD = 625,
    (HidIoCommandId::Unused, 0),              // TODO KEY_PAUSE_RECORD = 626,
    (HidIoCommandId::Unused, 0),              // TODO KEY_VOD = 627,
    (HidIoCommandId::Unused, 0),              // TODO KEY_UNMUTE = 628,
    (HidIoCommandId::Unused, 0),              // TODO KEY_FASTREVERSE = 629,
    (HidIoCommandId::Unused, 0),              // TODO KEY_SLOWREVERSE = 630,
    (HidIoCommandId::Unused, 0),              // TODO KEY_DATA = 631,
    (HidIoCommandId::Unused, 0),              // TODO KEY_ONSCREEN_KEYBOARD = 632,
    (HidIoCommandId::Unused, 0),              // TODO KEY_MAX = 767,
    (HidIoCommandId::Unused, 0),              // TODO BTN_0 = 256,
    (HidIoCommandId::Unused, 0),              // TODO BTN_1 = 257,
    (HidIoCommandId::Unused, 0),              // TODO BTN_2 = 258,
    (HidIoCommandId::Unused, 0),              // TODO BTN_3 = 259,
    (HidIoCommandId::Unused, 0),              // TODO BTN_4 = 260,
    (HidIoCommandId::Unused, 0),              // TODO BTN_5 = 261,
    (HidIoCommandId::Unused, 0),              // TODO BTN_6 = 262,
    (HidIoCommandId::Unused, 0),              // TODO BTN_7 = 263,
    (HidIoCommandId::Unused, 0),              // TODO BTN_8 = 264,
    (HidIoCommandId::Unused, 0),              // TODO BTN_9 = 265,
    (HidIoCommandId::Unused, 0),              // TODO BTN_LEFT = 272,
    (HidIoCommandId::Unused, 0),              // TODO BTN_RIGHT = 273,
    (HidIoCommandId::Unused, 0),              // TODO BTN_MIDDLE = 274,
    (HidIoCommandId::Unused, 0),              // TODO BTN_SIDE = 275,
    (HidIoCommandId::Unused, 0),              // TODO BTN_EXTRA = 276,
    (HidIoCommandId::Unused, 0),              // TODO BTN_FORWARD = 277,
    (HidIoCommandId::Unused, 0),              // TODO BTN_BACK = 278,
    (HidIoCommandId::Unused, 0),              // TODO BTN_TASK = 279,
    (HidIoCommandId::Unused, 0),              // TODO BTN_TRIGGER = 288,
    (HidIoCommandId::Unused, 0),              // TODO BTN_THUMB = 289,
    (HidIoCommandId::Unused, 0),              // TODO BTN_THUMB2 = 290,
    (HidIoCommandId::Unused, 0),              // TODO BTN_TOP = 291,
    (HidIoCommandId::Unused, 0),              // TODO BTN_TOP2 = 292,
    (HidIoCommandId::Unused, 0),              // TODO BTN_PINKIE = 293,
    (HidIoCommandId::Unused, 0),              // TODO BTN_BASE = 294,
    (HidIoCommandId::Unused, 0),              // TODO BTN_BASE2 = 295,
    (HidIoCommandId::Unused, 0),              // TODO BTN_BASE3 = 296,
    (HidIoCommandId::Unused, 0),              // TODO BTN_BASE4 = 297,
    (HidIoCommandId::Unused, 0),              // TODO BTN_BASE5 = 298,
    (HidIoCommandId::Unused, 0),              // TODO BTN_BASE6 = 299,
    (HidIoCommandId::Unused, 0),              // TODO BTN_DEAD = 303,
    (HidIoCommandId::Unused, 0),              // TODO BTN_SOUTH = 304,
    (HidIoCommandId::Unused, 0),              // TODO BTN_EAST = 305,
    (HidIoCommandId::Unused, 0),              // TODO BTN_C = 306,
    (HidIoCommandId::Unused, 0),              // TODO BTN_NORTH = 307,
    (HidIoCommandId::Unused, 0),              // TODO BTN_WEST = 308,
    (HidIoCommandId::Unused, 0),              // TODO BTN_Z = 309,
    (HidIoCommandId::Unused, 0),              // TODO BTN_TL = 310,
    (HidIoCommandId::Unused, 0),              // TODO BTN_TR = 311,
    (HidIoCommandId::Unused, 0),              // TODO BTN_TL2 = 312,
    (HidIoCommandId::Unused, 0),              // TODO BTN_TR2 = 313,
    (HidIoCommandId::Unused, 0),              // TODO BTN_SELECT = 314,
    (HidIoCommandId::Unused, 0),              // TODO BTN_START = 315,
    (HidIoCommandId::Unused, 0),              // TODO BTN_MODE = 316,
    (HidIoCommandId::Unused, 0),              // TODO BTN_THUMBL = 317,
    (HidIoCommandId::Unused, 0),              // TODO BTN_THUMBR = 318,
    (HidIoCommandId::Unused, 0),              // TODO BTN_TOOL_PEN = 320,
    (HidIoCommandId::Unused, 0),              // TODO BTN_TOOL_RUBBER = 321,
    (HidIoCommandId::Unused, 0),              // TODO BTN_TOOL_BRUSH = 322,
    (HidIoCommandId::Unused, 0),              // TODO BTN_TOOL_PENCIL = 323,
    (HidIoCommandId::Unused, 0),              // TODO BTN_TOOL_AIRBRUSH = 324,
    (HidIoCommandId::Unused, 0),              // TODO BTN_TOOL_FINGER = 325,
    (HidIoCommandId::Unused, 0),              // TODO BTN_TOOL_MOUSE = 326,
    (HidIoCommandId::Unused, 0),              // TODO BTN_TOOL_LENS = 327,
    (HidIoCommandId::Unused, 0),              // TODO BTN_TOOL_QUINTTAP = 328,
    (HidIoCommandId::Unused, 0),              // TODO BTN_STYLUS3 = 329,
    (HidIoCommandId::Unused, 0),              // TODO BTN_TOUCH = 330,
    (HidIoCommandId::Unused, 0),              // TODO BTN_STYLUS = 331,
    (HidIoCommandId::Unused, 0),              // TODO BTN_STYLUS2 = 332,
    (HidIoCommandId::Unused, 0),              // TODO BTN_TOOL_DOUBLETAP = 333,
    (HidIoCommandId::Unused, 0),              // TODO BTN_TOOL_TRIPLETAP = 334,
    (HidIoCommandId::Unused, 0),              // TODO BTN_TOOL_QUADTAP = 335,
    (HidIoCommandId::Unused, 0),              // TODO BTN_GEAR_DOWN = 336,
    (HidIoCommandId::Unused, 0),              // TODO BTN_GEAR_UP = 337,
    (HidIoCommandId::Unused, 0),              // TODO BTN_DPAD_UP = 544,
    (HidIoCommandId::Unused, 0),              // TODO BTN_DPAD_DOWN = 545,
    (HidIoCommandId::Unused, 0),              // TODO BTN_DPAD_LEFT = 546,
    (HidIoCommandId::Unused, 0),              // TODO BTN_DPAD_RIGHT = 547,
    (HidIoCommandId::Unused, 0),              // TODO BTN_TRIGGER_HAPPY1 = 704,
    (HidIoCommandId::Unused, 0),              // TODO BTN_TRIGGER_HAPPY2 = 705,
    (HidIoCommandId::Unused, 0),              // TODO BTN_TRIGGER_HAPPY3 = 706,
    (HidIoCommandId::Unused, 0),              // TODO BTN_TRIGGER_HAPPY4 = 707,
    (HidIoCommandId::Unused, 0),              // TODO BTN_TRIGGER_HAPPY5 = 708,
    (HidIoCommandId::Unused, 0),              // TODO BTN_TRIGGER_HAPPY6 = 709,
    (HidIoCommandId::Unused, 0),              // TODO BTN_TRIGGER_HAPPY7 = 710,
    (HidIoCommandId::Unused, 0),              // TODO BTN_TRIGGER_HAPPY8 = 711,
    (HidIoCommandId::Unused, 0),              // TODO BTN_TRIGGER_HAPPY9 = 712,
    (HidIoCommandId::Unused, 0),              // TODO BTN_TRIGGER_HAPPY10 = 713,
    (HidIoCommandId::Unused, 0),              // TODO BTN_TRIGGER_HAPPY11 = 714,
    (HidIoCommandId::Unused, 0),              // TODO BTN_TRIGGER_HAPPY12 = 715,
    (HidIoCommandId::Unused, 0),              // TODO BTN_TRIGGER_HAPPY13 = 716,
    (HidIoCommandId::Unused, 0),              // TODO BTN_TRIGGER_HAPPY14 = 717,
    (HidIoCommandId::Unused, 0),              // TODO BTN_TRIGGER_HAPPY15 = 718,
    (HidIoCommandId::Unused, 0),              // TODO BTN_TRIGGER_HAPPY16 = 719,
    (HidIoCommandId::Unused, 0),              // TODO BTN_TRIGGER_HAPPY17 = 720,
    (HidIoCommandId::Unused, 0),              // TODO BTN_TRIGGER_HAPPY18 = 721,
    (HidIoCommandId::Unused, 0),              // TODO BTN_TRIGGER_HAPPY19 = 722,
    (HidIoCommandId::Unused, 0),              // TODO BTN_TRIGGER_HAPPY20 = 723,
    (HidIoCommandId::Unused, 0),              // TODO BTN_TRIGGER_HAPPY21 = 724,
    (HidIoCommandId::Unused, 0),              // TODO BTN_TRIGGER_HAPPY22 = 725,
    (HidIoCommandId::Unused, 0),              // TODO BTN_TRIGGER_HAPPY23 = 726,
    (HidIoCommandId::Unused, 0),              // TODO BTN_TRIGGER_HAPPY24 = 727,
    (HidIoCommandId::Unused, 0),              // TODO BTN_TRIGGER_HAPPY25 = 728,
    (HidIoCommandId::Unused, 0),              // TODO BTN_TRIGGER_HAPPY26 = 729,
    (HidIoCommandId::Unused, 0),              // TODO BTN_TRIGGER_HAPPY27 = 730,
    (HidIoCommandId::Unused, 0),              // TODO BTN_TRIGGER_HAPPY28 = 731,
    (HidIoCommandId::Unused, 0),              // TODO BTN_TRIGGER_HAPPY29 = 732,
    (HidIoCommandId::Unused, 0),              // TODO BTN_TRIGGER_HAPPY30 = 733,
    (HidIoCommandId::Unused, 0),              // TODO BTN_TRIGGER_HAPPY31 = 734,
    (HidIoCommandId::Unused, 0),              // TODO BTN_TRIGGER_HAPPY32 = 735,
    (HidIoCommandId::Unused, 0),              // TODO BTN_TRIGGER_HAPPY33 = 736,
    (HidIoCommandId::Unused, 0),              // TODO BTN_TRIGGER_HAPPY34 = 737,
    (HidIoCommandId::Unused, 0),              // TODO BTN_TRIGGER_HAPPY35 = 738,
    (HidIoCommandId::Unused, 0),              // TODO BTN_TRIGGER_HAPPY36 = 739,
    (HidIoCommandId::Unused, 0),              // TODO BTN_TRIGGER_HAPPY37 = 740,
    (HidIoCommandId::Unused, 0),              // TODO BTN_TRIGGER_HAPPY38 = 741,
    (HidIoCommandId::Unused, 0),              // TODO BTN_TRIGGER_HAPPY39 = 742,
    (HidIoCommandId::Unused, 0),              // TODO BTN_TRIGGER_HAPPY40 = 743,

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
fn evdev2basehid(code: evdev_rs::enums::EventCode) -> std::io::Result<(HidIoCommandId, u16)> {
    use evdev_rs::enums::EventCode;
    match code.clone() {
        EventCode::EV_KEY(key) => {
            // Do an ev code to hid code lookup
            // Will error if no lookup is available
            let key = key as usize;
            let lookup = EVDEV2HIDKEY[key];
            if lookup.0 == HidIoCommandId::Unused {
                Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!(
                        "No key hid code lookup for ev code: {} {:?} {}",
                        code, key, lookup.1
                    ),
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

    /// Process evdev events
    /// NOTE: evdev doesn't necessarily group all event codes from a single Hid message into a
    /// single EV_SYN scan report. While annoying (and makes it hard to perfectly emulate hid) this
    /// is how normal NKRO keyboards are also handled on Linux so users won't notice a difference.
    /// On each scan report additional keys will be added to the HidIo packet so you'll eventually
    /// get the full set (just communication more "chatty"). This also complicates unit testing :/
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
        info!("Connection event uid:{} {}", self.uid, device_name(&device));

        // Take all event information (block events from other processes)
        device.grab(evdev_rs::GrabMode::Grab).unwrap();

        // Queue up evdev events to send
        // Each event is received individually, but we want all events that come from an
        // instance in time (in order to emulate how hid devices send devices; as well as how
        // HidIo packets send state)
        let mut event_queue: Vec<evdev_rs::InputEvent> = vec![];
        let mut event_queue_command = HidIoCommandId::HidKeyboard; // Default to a keyboard message
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
                    "uid:{} {:?} {:?} {}",
                    self.uid, &result.1.event_type, &result.1.event_code, &result.1.value
                );

                match result.0 {
                    evdev_rs::ReadStatus::Sync => {
                        // Dropped packet (this shouldn't happen)
                        // We should warn about it though
                        warn!("Dropped evdev event! - Attempting to resync...");
                        while result.0 == evdev_rs::ReadStatus::Sync {
                            warn!(
                                "Dropped: uid:{} {:?} {:?} {}",
                                self.uid,
                                &result.1.event_type,
                                &result.1.event_code,
                                &result.1.value
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
                                    // Generate HidIo packet data
                                    let data = match event_queue_command {
                                        HidIoCommandId::HidKeyboard => {
                                            // Convert evdev codes into base hid codes
                                            let mut data = vec![];
                                            for event in event_queue.clone() {
                                                let code = event.event_code;
                                                match evdev2basehid(code) {
                                                    Ok(code) => {
                                                        // TODO Handle SystemCtrl and ConsumerCtrl
                                                        if code.0 == HidIoCommandId::HidKeyboard {
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
                                            debug!(
                                                "Ignoring send: uid:{} {:?}",
                                                self.uid, event_queue
                                            );
                                            continue;
                                        }
                                    };

                                    // Encode the message as a HidIo packet
                                    self.mailbox
                                        .try_send_command(
                                            mailbox::Address::DeviceHid { uid: self.uid },
                                            mailbox::Address::All,
                                            event_queue_command,
                                            data,
                                            false,
                                        )
                                        .unwrap();
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

                        // Select the type of HidIo Packet being sent based off of the device type
                        event_queue_command = match self.endpoint.type_() {
                            common_capnp::NodeType::HidKeyboard => {
                                // Filter for keyboard events
                                if !&result.1.is_type(&evdev_rs::enums::EventType::EV_KEY) {
                                    continue;
                                }
                                HidIoCommandId::HidKeyboard
                            }
                            common_capnp::NodeType::HidMouse => {
                                // Filter for mouse events
                                // TODO
                                // TODO We may need to handle more complicated mouse packets
                                HidIoCommandId::HidMouse
                            }
                            common_capnp::NodeType::HidJoystick => {
                                // Filter for joystick events
                                // TODO
                                // TODO We may need to handle more complicated joystick packets
                                HidIoCommandId::HidJoystick
                            }
                            _ => {
                                panic!(
                                    "Unknown type for EvdevDevice endpoint node: {:?}",
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
                        info!(
                            "Disconnection event uid:{} {}",
                            self.uid,
                            device_name(&device)
                        );
                        return Ok(());
                    }
                }
            }

            // TODO Check if there are more events, if yes, keep trying to enqueue
        }
    }
}

impl Drop for EvdevDevice {
    fn drop(&mut self) {
        // Unregister node
        self.mailbox.unregister_node(self.uid);
    }
}

/// Build a unique device name string
fn device_name(device: &evdev_rs::Device) -> String {
    let string = format!(
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

    if device.has(&EventCode::EV_KEY(EV_KEY::KEY_F))
        || device.has(&EventCode::EV_KEY(EV_KEY::KEY_J))
    {
        Ok(common_capnp::NodeType::HidKeyboard)
    } else if device.has(&EventCode::EV_KEY(EV_KEY::BTN_LEFT)) {
        Ok(common_capnp::NodeType::HidMouse)
    } else if device.has(&EventCode::EV_KEY(EV_KEY::BTN_TRIGGER)) {
        Ok(common_capnp::NodeType::HidJoystick)
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("{} is not a keyboard, mouse or joystick", fd_path),
        ))
    }
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

    // Initialize Hid interface
    let mut api = ::hidapi::HidApi::new().expect("Hid API object creation failed");

    let mut devices: Vec<HidIoController> = vec![];

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

                // Use usage page and usage for matching  compatible device
                if !match_device(device_info) {
                    continue;
                }

                // Build set of Hid info to make unique comparisons
                let mut info = HidApiInfo::new(device_info);

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
                        let device = HidApiDevice::new(device);
                        let mut device =
                            HidIoEndpoint::new(Box::new(device), USB_FULLSPEED_PACKET_SIZE as u32);

                        if let Err(e) = device.send_sync() {
                            // Could not open device (likely removed, or in use)
                            warn!("Processing - {}", e);
                            continue;
                        }

                        // Setup device controller (handles communication and protocol conversion
                        // for the HidIo device)
                        let master = HidIoController::new(mailbox.clone(), uid, device);
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
                tokio::time::sleep(std::time::Duration::from_millis(ENUMERATE_DELAY)).await;
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
                tokio::time::sleep(std::time::Duration::from_millis(POLL_DELAY)).await;
            }
        }
    }
}
*/

/// Supported Ids by this module
pub fn supported_ids() -> Vec<HidIoCommandId> {
    let ids: Vec<HidIoCommandId> = vec![];
    ids
}

/// evdev initialization
///
/// Sets up processing threads for udev and evdev.
pub async fn initialize(_mailbox: mailbox::Mailbox) {
    info!("Initializing device/evdev...");

    // Spawn watcher thread (tokio)
    // TODO - udev monitoring (waiting for devices to reconnect)
    // TODO - evev monitoring (monitoring is done by api request, grabbing is an option)
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

#[cfg(test)]
mod test {
    use super::*;
    use crate::logging::setup_logging_lite;
    use std::sync::{Arc, RwLock};

    #[test]
    #[ignore]
    fn uhid_evdev_keyboard_test() {
        setup_logging_lite().ok();
        // Create uhid keyboard interface
        let name = "evdev-keyboard-nkro-test".to_string();
        let mailbox = mailbox::Mailbox {
            ..Default::default()
        };

        // Adjust next uid to make it easier to debug parallel tests
        *mailbox.last_uid.write().unwrap() = 10;

        // Generate a unique key (to handle parallel tests)
        let uniq = nanoid::nanoid!();

        // Instantiate hid device
        let mut keyboard = vhid::uhid::KeyboardNkro::new(
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

        let rt = tokio::runtime::Runtime::new().unwrap();
        let status: Arc<RwLock<bool>> = Arc::new(RwLock::new(false));
        let status2 = status.clone();

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
                            *(status.clone().write().unwrap()) = true;
                            continue;
                        }

                        // Verify the incoming keypresses
                        if msg.data.data.to_vec() == expected_msgs[msg_pos] {
                            msg_pos += 1;
                        } else {
                            assert!(
                                msg.data.data.to_vec() == vec![],
                                "Unexpected message: {:?}",
                                msg
                            );
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        assert!(false, "Mailbox has been closed unexpectedly!");
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
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

        rt.block_on(async {
            // Make sure everything is initialized and monitoring
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;

            // Send A;A,B;B key using uhid device
            // TODO integrate layouts-rs from  (to have symbolic testing inputs)
            keyboard.send(vec![4]).unwrap();
            keyboard.send(vec![4, 5]).unwrap();
            keyboard.send(vec![5]).unwrap();
            keyboard.send(vec![]).unwrap();

            // Give some time for the events to propagate
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        });

        // Force the runtime to shutdown
        rt.shutdown_timeout(std::time::Duration::from_millis(100));
        let status: bool = *status2.clone().read().unwrap();
        assert!(status, "Test failed");
    }
}
