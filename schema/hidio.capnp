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

@0xd525cce96cb24671;

## Imports ##

using Common = import "common.capnp";

using import "hidiowatcher.capnp".HIDIOWatcher;
using import "hostmacro.capnp".HostMacro;
using import "usbkeyboard.capnp".USBKeyboard;



## Interfaces ##

interface HIDIOServer {
    # Authentication interface for HIDIO

    basic @0 (info :Common.Source) -> (port :HIDIO);
    # Allocates a basic interface, with no special priviledges

    # TODO Add authentication schemes
    auth @1 (info :Common.Source) -> (port :HIDIO);
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
}

