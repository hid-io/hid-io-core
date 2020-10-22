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

@0xe10f671900e093b0;

## Imports ##

using Common = import "common.capnp";
using HidIo = import "hidio.capnp";



## Interfaces ##

interface Keyboard extends(HidIo.Node) {
    # HidIo Keyboard node

    enum SubscriptionOptionType {
        hostMacro @0;
        # Host Macro subscription option
        # A host macro supplies an index that can be assigned to an arbitrary function

        layer @1;
        # Layer subscription option
        # Sends notifications on layer changes

        kllTrigger @2;
        # KLL trigger subscription option
        # Scan Code subscription
        # This uses the devices internal numbering scheme which must be converted to HID in order to use
        # Only returns a list of activated scan codes
        # If no id/index pairs are specified, this subscribes to all KLL triggers

        kllTriggerDisable @3;
        # KLL trigger disable option
        # This specifies which KLL Triggers are ignored by the keyboard's internal macro engine
        # (and will not be sent over the HID keyboard interface).
        # Useful when using the API to send data directly to the application or through a virtual HID device
        # If no id/index pairs are specified, all kllTriggers are ignored by the keyboard.

        cliOutput @4;
        # Subscribe to CLI output
        # Useful when using cli commands (to see the result) or to monitor keyboard status and internal errors
    }


    struct Signal {
        struct Cli {
            # Cli Message output text
            output @0 :Text;
        }

        struct KLL {
        }

        struct HostMacro {
        }

        struct Layer {
        }

        time @0 :UInt64;
        # Signal event timestamp

        source @1 :Common.Source;
        # Source of signal

        data :union {
            cli @2 :Cli;
            # CLI Output message

            kll @3 :KLL;
            # KLL Trigger message

            hostMacro @4 :HostMacro;
            # Host Macro message

            layer @5 :Layer;
            # Layer event message
        }
    }

    struct SubscriptionOption {
        type @0 :SubscriptionOptionType;

        struct KLLTriggerOption {
            id @0 :UInt8;
            # This maps to a KLL TriggerType
            # https://github.com/kiibohd/controller/blob/master/Macro/PartialMap/kll.h#L263

            index @1 :UInt8;
            # This maps to a KLL TriggerEvent index (i.e. ScanCode)
            # This number is always 8-bits, for higher scancodes you'll need to use a different id
            # See https://github.com/kiibohd/controller/blob/master/Macro/PartialMap/kll.h#L221
        }

        struct NoneOption {}

        conf :union {
            kllTrigger @1 :List(KLLTriggerOption);
            # Specified with a kllTrigger or kllTriggerDisable option

            none @2 :NoneOption;
            # For options that don't need further configuration
        }
    }


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

    # TODO
    # Scan Code -> HID Code lookup (per layer)
    # Pixel Control
    # Generic commands
}
