# Copyright (C) 2017 by Jacob Alexander
#
# This file is free software: you can redistribute it and/or modify
# it under the terms of the GNU General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This file is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License
# along with this file.  If not, see <http://www.gnu.org/licenses/>.

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

