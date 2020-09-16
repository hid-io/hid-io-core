# Copyright (C) 2020 by Jacob Alexander
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

@0x89950abd85ed15de;

## Imports ##

using Common = import "common.capnp";



## Interfaces ##

interface Daemon extends(Common.Node) {
    # API interface to hid-io-core
    # This is the main entry point for calling hid-io-core functionality.

    struct Signal {
        time @0 :UInt64;
        # Signal event timestamp

        source @1 :Common.Source;
        # Source of signal

        # TODO
    }

    struct SubscriptionOption {}


    interface Subscription {
        # Subscription interface
        # Handles subscription ownership and when to drop subscription
    }

    interface Subscriber {
        # Node subscriber
        # Handles any push notifications from hid-io-core endpoints
        # NOTE: Not all packets are sent by default, you must configure the subscription to enable the ones you want

        update @0 (time :UInt64, signal :List(Signal));
        # Called whenever a subscribed packet type (to this device) is available
        # May return 1 or more packets depending on the size of the queue
        #
        # Time is when the rpc is sent.
        # Useful when determining Signal ordering
    }

    subscribe @0 (subscriber :Subscriber, options :List(SubscriptionOption)) -> (subscription :Subscription);
    # Subscribes to a Subscriber interface
    # Registers push notifications for this node, the packets received will depend on the SubscriptionOption list
    # By default no packets will be sent
    # Will return an error if any of the options are not supported/invalid for this device

    # Unicode
    # TODO
    # String
    # Symbol
    # Get Held
    # Set Held

    # Layout
    # Get
    # Set

    # VirtualKeyboard
    # VirtualMouse
    # VirtualJoystick
}
