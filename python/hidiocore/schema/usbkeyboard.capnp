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

