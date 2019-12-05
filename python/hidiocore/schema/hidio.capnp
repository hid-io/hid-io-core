# Copyright (C) 2017-2019 by Jacob Alexander
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

@0xd525cce96cb24671;

## Imports ##

using Common = import "common.capnp";

using import "hidiowatcher.capnp".HIDIOWatcher;
using import "hostmacro.capnp".HostMacro;
using import "usbkeyboard.capnp".USBKeyboard;



## Interfaces ##

interface HIDIOServer {
    # Authentication interface for HIDIO

    struct Version {
        version @0 :Text;
        # Version number of running server

        buildtime @1 :Text;
        # Date server was build

        serverarch @2 :Text;
        # Architecture of running server

        compilerversion @3 :Text;
        # Version of the compiler used to build server
    }

    struct KeyInfo {
        basicKeyPath @0 :Text;
        # File path to basic key
        # Has permissions of basic user

        authKeyPath @1 :Text;
        # File path to authenticated key
        # Must have root/admin priviledges to read this key
    }

    basic @0 (info :Common.Source, key :Text) -> (port :HIDIO);
    # Allocates a basic interface, with no special priviledges
    # Must include a key retrieved using locations specified by HIDIOInit

    auth @1 (info :Common.Source, key :Text) -> (port :HIDIO);
    # Priviledged interface
    # Must include a key retrieved using locations specified by HIDIOInit

    version @2 () -> (version :Version);
    # Returns the version number of the running server

    key @3 () -> (key :KeyInfo);
    # Returns information needed to authenticate with HIDIOServer

    alive @4 () -> (alive: Bool);
    # Always returns true, used to determine if socket connection/API is working

    id @5 () -> (id :UInt64);
    # Unique id
    # Assigned per socket connection
    # This must be used when attempting basic/auth authentication

    name @6 () -> (name :Text);
    # Name of HID-IO Server

    logFiles @7 () -> (paths :List(Text));
    # Path to the local hid-io core log file(s)
    # rCURRENT is the current active log file
}

interface HIDIO {
    # Main HIDIO Interface
    # Requires authentication through HIDIOServer first
    struct Signal {
        time @0 :UInt64;
        # Signal event timestamp

        source @1 :Common.Source;
        # Source of signal

        type :union {
            usbKeyboard @2 :USBKeyboard.Signal;
            hostMacro @3 :HostMacro.Signal;
            hidioPacket @4 :HIDIOWatcher.Signal;
        }
        # Signal packet information
        # Each module's signal struct further specializes the return value
    }

    signal @0 (time :UInt64) -> (time :UInt64, signal :List(Signal));
    # Promise subscription
    # After each return, call again
    # Will return when there is information to send
    # You will have to use the nodes() function to register with each of the possible signals
    #
    # Use 0 as the starting time
    # Time is unix time
    # The return time is the time the signal list starts
    # This may be the same time, or earlier than the first signal, to signify nothing came before it

    nodes @1 () -> (nodes :List(Common.Destination));
    # List of supported nodes
    # This may not contain all nodes due to authentication levels
    # The HIDIO daemon may revoke priviledges for certain modules during runtime

    interface NodesSubscription {}
    # Node subscription interface
    # Handles subscription ownership and when to drop subscription

    interface NodesSubscriber {
        # Client node subscriber
        # Handles any push methods that hid-io-core can send

        nodesUpdate @0 (nodes :List(Common.Destination));
        # Called whenever the list of nodes changes
    }

    subscribeNodes @2 (subscriber :NodesSubscriber) -> (subscription :NodesSubscription);
    # Subscribes a NodesSubscriber interface
    # Registers push notifications for node list changes
}

