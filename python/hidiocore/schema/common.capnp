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

using import "hidiowatcher.capnp".HIDIOWatcher;
using import "hostmacro.capnp".HostMacro;
using import "usbkeyboard.capnp".USBKeyboard;

## Enumerations ##

enum KeyEventState {
    off @0;
    press @1;
    hold @2;
    release @3;
}

enum NodeType {
    hidioDaemon @0;
    hidioApi @1;
    usbKeyboard @2;
    bleKeyboard @3;
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

    node @4 :HIDIONode;
    # Interface node of destination
    # A separate node is generated for each interface node
    # (i.e. there may be multiple nodes per physical/virtual device)

    commands :union {
        usbKeyboard @5 :USBKeyboard.Commands;
	hostMacro @6 :HostMacro.Commands;
	hidioPacket @7 :HIDIOWatcher.Commands;
    }
}



## Interfaces ##

interface HIDIONode {
    # Common interface for all module nodes for HIDIO

    register @0 () -> (ok :Bool);
    # Register signal with daemon

    isRegistered @1 () -> (ok :Bool);
    # Returns on whether the node is registered with your client for signalling
    #
}

