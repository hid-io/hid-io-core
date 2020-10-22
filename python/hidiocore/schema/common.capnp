# Copyright (C) 2017-2020 by Jacob Alexander
#
# Permission is hereby granted, free of charge, to any person obtaining a copy
# of this software and associated documentation files (the "Software"), to deal
# in the Software without restriction, including without limitation the rights
# to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
# copies of the Software, and to permit persons to whom the Software is
# furnished to do so, subject to the following conditions:
#
# The above copyright notice and this permission notice shall be included in all
# copies or substantial portions of the Software.
#
# THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
# IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
# FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
# AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
# LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
# OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
# SOFTWARE.

@0xeed5743611d4cb4a;

## Imports ##

using import "daemon.capnp".Daemon;
using import "keyboard.capnp".Keyboard;



## Enumerations ##

enum NodeType {
    hidioDaemon @0;
    # HidIo Daemon node (hid-io-core functionality)
    # There should always be a single daemon node active

    hidioApi @1;
    # Generic HidIo API node

    usbKeyboard @2;
    # HidIo USB Keyboard

    bleKeyboard @3;
    # HidIo BLE Keyboard

    hidKeyboard @4;
    # Generic HID Keyboard

    hidMouse @5;
    # Generic HID Mouse

    hidJoystick @6;
    # Generic HID Joystick
}
# Node types, please extend this enum as necessary
# Should be generic types, nothing specific, use the text field for that



## Structs ##

struct Source {
    # This struct represents the source of a signal

    type @0 :NodeType;
    # Type of node

    name @1 :Text;
    # Name of source
    # May or may not be unique

    serial @2 :Text;
    # Serial number (text field) of source
    # Zero-length if unused

    id @3 :UInt64;
    # Unique id identifying source
    # While daemon is running, ids are only reused once 2^64 Ids have been utilized.
    # This allows (for at least a brief period) a unique source (which may disappear before processing)
}

struct Destination {
    # This struct represents destination of a function
    # Not all functions require a destination

    type @0 :NodeType;
    # Type of node

    name @1 :Text;
    # Name of destination
    # May or may not be unique

    serial @2 :Text;
    # Serial number (text field) of destination
    # Zero-length if unused

    id @3 :UInt64;
    # Unique id identifying destination
    # While daemon is running, ids are only reused once 2^64 Ids have been utilized.
    # This allows (for at least a brief period) a unique source (which may disappear before processing)

    node :union {
        # Interface node of destination
        # A separate node is generated for each interface node
        # (i.e. there may be multiple nodes per physical/virtual device)
        #
        # May not be set depending on the type of node as there may not be any additional functionality available
        # to the API

        keyboard @4 :Keyboard;
        # HidIo Keyboard Node
        # Valid when the type is set to usbKeyboard or bleKeyboard

        daemon @5 :Daemon;
        # Daemon (hid-io-core) Command Node
        # Valid when the type is set to hidioDaemon
    }
}



## Interfaces ##

interface Node {
    # Common interface for all HidIo api nodes
}
