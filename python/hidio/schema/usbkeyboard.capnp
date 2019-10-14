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

@0xe10f671900e093b0;

## Imports ##

using Common = import "common.capnp";



## Interfaces ##

interface USBKeyboard extends(Common.HIDIONode) {
    struct KeyEvent {
        event @0 :Common.KeyEventState;
        id @1 :UInt16;
    }

    struct KeyEventStatus {
        event @0 :Common.KeyEventState;
        id @1 :UInt16;
        success @2 :Bool;
    }

    struct Signal {
        union {
            keyEvent @0 :KeyEvent;
            scanCodeEvent @1 :KeyEvent;
        }
        # Signals on each USB key event change
        # XXX (HaaTa) This can be used to build a keylogger, should only be allowed for priviledged interfaces
    }

    interface Commands {
	    keyEvent @0 (events :List(KeyEvent)) -> (status :List(KeyEventStatus));
	    # Send USB Key Event to keyboard to process
	    # NOTE (HaaTa) Not available for Scan Codes, as they should reflect the actual hardware state
	    #              Whereas USB Codes may be the result of macros and/or layers.

	    cliCommand @1 (foobar :Text) -> ();
    }
}

