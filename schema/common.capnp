# Copyright (C) 2017-2019 by Jacob Alexander
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

using import "blekeyboard.capnp".BLEKeyboardInfo;
using import "blemouse.capnp".BLEMouseInfo;
using import "usbkeyboard.capnp".USBKeyboardInfo;
using import "usbmouse.capnp".USBMouseInfo;



## Enumerations ##

enum KeyEventState {
    off @0;
    press @1;
    hold @2;
    release @3;
}

enum NodeType {
    bleKeyboard @4;
    bleMouse @5;
    hidioDaemon @0;
    hidioScript @1;
    usbKeyboard @2;
    usbMouse @3;
    unknown @6;
}
# Node types, please extend this enum as necessary
# Should be generic types, nothing specific, use the text field for that

enum EventState {
    invalid @0;      # Invalid, id does not exist
    connected @1;    # Connected and active
    hung @2;         # Active, but has not responded with SYNCs in a while
    inactive @3;     # No longer connected and considered inactive
    reap @4;         # Set to be cleaned up and resources freed
    notconnected @5; # Not connected, but have seen
}
# These states represent the status of device/api endpoint connections
# The states indicate how the Info struct should be interpreted



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

struct Event {
    # Event container struct
    # Indicates what happened and when

    state @0 :EventState;
    # State of this event

    time @1 :UInt64;
    # Timestamp of this event (unix time)
}

struct Info {
    # Info container struct
    # Depending on the type, a different struct will be available
    # TODO (HaaTa): It may be useful to provide a function that can refresh info

    event @0 :List(Event);
    # Indicates how this Info struct should be interpreted
    # If set to invalid, none of the other fields will contain useful data
    # The last element of this list is the most recent (i.e. in chronological order)

    type @1 :NodeType;
    # Type of node this info struct is for

    info :union {
        usbKeyboard @2 :USBKeyboardInfo;
	usbMouse @3 :USBMouseInfo;
	bleKeyboard @4 :BLEKeyboardInfo;
	bleMouse @5 :BLEMouseInfo;
    }
    # Info container union
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

