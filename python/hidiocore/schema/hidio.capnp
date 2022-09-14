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

@0xd525cce96cb24671;

## Imports ##

using Common = import "common.capnp";



## Interfaces ##

interface HidIoServer {
    # Authentication interface for HidIo

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

    ## Functions ##

    basic @0 (info :Common.Source, key :Text) -> (port :HidIo);
    # Allocates a basic interface, with no special priviledges
    # Must include a key retrieved using locations specified by HidIoInit

    auth @1 (info :Common.Source, key :Text) -> (port :HidIo);
    # Priviledged interface
    # Must include a key retrieved using locations specified by HidIoInit

    version @2 () -> (version :Version);
    # Returns the version number of the running server

    key @3 () -> (key :KeyInfo);
    # Returns information needed to authenticate with HidIoServer

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

interface HidIo {
    # Main HidIo Interface
    # Requires authentication through HidIoServer first

    struct Packet {
        # This struct represents a modified HidIo packet
        # as used internally by hid-io-core.
        # This is not the same as the "on-the-wire" HidIo packets
        # (Continued packets are joined together)
        enum Type {
            data @0;
            # Data packet
            ack @1;
            # Ack for a data packet
            nak @2;
            # Nak for a data packet (Error)
            naData @3;
            # Non-acknowledged data packet (no corresponding ack/nak packet)
            unknown @4;
            # Unknown packet type (i.e. there's a bug somewhere)
        }

        src @0 :UInt64;
        # Source uid of the packet (set to 0 if N/A)

        dst @1 :UInt64;
        # Destination uid of the packet (set to 0 if N/A)

        type @2 :Type;
        # Type of HidIo packet

        id @3 :UInt32;
        # Id of the HidIo packet

        data @4 :List(UInt8);
        # Payload data of packet (in bytes)
    }

    nodes @0 () -> (nodes :List(Common.Destination));
    # List of supported nodes
    # This may not contain all nodes due to authentication levels
    # The HidIo daemon may revoke priviledges for certain modules during runtime

    interface NodesSubscription {}
    # Node subscription interface
    # Handles subscription ownership and when to drop subscription

    interface NodesSubscriber {
        # Client node subscriber
        # Handles any push methods that hid-io-core can send

        nodesUpdate @0 (nodes :List(Common.Destination));
        # Called whenever the list of nodes changes

        hidioWatcher @1 (packet :Packet);
        # Called on every internal HidIo message
        # This watcher will show most of the "on-the-wire" packets as well as some hid-io-core internal packets.
        # Sync, Continued and NAContinued will not be triggered by the watcher.
        # NOTE: This callback is only used when hid-io-core is in debug mode with a priviledged interface
    }

    subscribeNodes @1 (subscriber :NodesSubscriber) -> (subscription :NodesSubscription);
    # Subscribes a NodesSubscriber interface
    # Registers push notifications for node list changes
}

interface Node extends(Common.Node) {
    # Common interface for all HidIo nodes

    struct FlashModeStatus {
        # Result of a flash mode command

        struct Success {
            scanCode @0 :UInt16;
            # In order to enter flash mode a specific (randomly) generated physical key must be pressed
            # This scan code refers to that physical key.
            # Use the key layout to determine the HID key label.
        }
        struct Error {
            # Entering flash mode failed

            reason @0 :ErrorReason;
            # Reason for flash mode failure

            enum ErrorReason {
                notSupported @0;
                # Flash mode is not supported on this device

                disabled @1;
                # Flash mode is disabled on this device (usually for security reasons)
            }
        }

        union {
            success @0 :Success;
            error @1 :Error;
        }
    }

    struct SleepModeStatus {
        # Result of a sleep mode command

        struct Success {}
        struct Error {
            # Entering sleep mode failed

            reason @0 :ErrorReason;
            # Reason for sleep mode failure

            enum ErrorReason {
                notSupported @0;
                # Sleep mode is not supported on this device

                disabled @1;
                # Sleep mode is disabled on this device

                notReady @2;
                # Not ready to enter sleep mode
                # This is usually due to some physical or USB state that is preventing the transition to sleep mode
            }
        }

        union {
            success @0 :Success;
            error @1 :Error;
        }
    }

    struct Manufacturing {
        enum Command {
            ledTestSequence @0;
            ledCycleKeypressTest @1;
            hallEffectSensorTest @2;
        }

        enum LedTestSequenceArg {
            disable @0;
            enable @1;
            activateLedShortTest @2;
            activateLedOpenCircuitTest @3;
        }

        enum LedCycleKeypressTestArg {
            disable @0;
            enable @1;
        }

        enum HallEffectSensorTestArg {
            disableAll @0;
            passFailTestToggle @1;
            levelCheckToggle @2;
        }

        command @0 :Command;

        union {
            ledTestSequence @1 :LedTestSequenceArg;
            ledCycleKeypressTest @2 :LedCycleKeypressTestArg;
            hallEffectSensorTest @3 :HallEffectSensorTestArg;
        }
    }

    struct ManufacturingStatus {
        struct Success {}
        struct Error {}

        # Result of manufacturing test command
        union {
            success @0 :Success;
            error @1 :Error;
        }
    }

    struct PixelSet {
        enum Type {
            directSet @0;
        }

        type @0 :Type;
        startAddress @1 :UInt16;
        directSetData @2 :Data;
    }

    struct PixelSetStatus {
        struct Success {}
        struct Error {}

        union {
            success @0 :Success;
            error @1 :Error;
        }
    }

    struct PixelSetting {
        enum Command {
            control @0;
            # General LED control commands

            reset @1;
            # LED controller reset modes

            clear @2;
            # Frame clearing commands

            frame @3;
            # Frame control commands
        }

        enum ControlArg {
            disable @0;
            # Disable HID-IO LED controller
            # This will usually give LED control back to the device and may disable the LEDs for some
            # devices

            enableStart @1;
            # Enables LED frame display in free running mode

            enablePause @2;
            # Enable LED frame display, frame: nextFrame must be called to iterate to the next frame.
        }

        enum ResetArg {
            softReset @0;
            # Clear current pixel frame and any settings

            hardReset @1;
            # Initiate a hard reset of the LED controller and initialize
            # default settings
        }

        enum ClearArg {
            clear @0;
            # Clear current pixel frame
            # Will need to iterate to the next frame if using EnablePause
        }

        enum FrameArg {
            nextFrame @0;
            # Iterate to next pixel buffer frame if using EnablePause
        }

        command @0 :Command;

        union {
            control @1 :ControlArg;
            reset @2 :ResetArg;
            clear @3 :ClearArg;
            frame @4 :FrameArg;
        }
    }

    struct PixelSettingStatus {
        struct Success {}
        struct Error {}

        union {
            success @0 :Success;
            error @1 :Error;
        }
    }

    struct Info {
        # Result of an info command

        hidioMajorVersion @0 :UInt16;
        hidioMinorVersion @1 :UInt16;
        hidioPatchVersion @2 :UInt16;
        # HID-IO Version information (supported version)

        deviceName @3 :Text;
        # Name of the device

        deviceVendor @9 :Text;
        # Name of the vendor of the device

        deviceSerial @4 :Text;
        # Serial number of the device

        deviceVersion @5 :Text;
        # Version of the device
        # (this is sometimes used if different firmware is necessary for different generations of the same device)

        deviceMcu @6 :Text;
        # MCU of the device (there maybe be multiple)

        firmwareName @7 :Text;
        # Name of the firmware (e.g. kiibohd, QMK, etc.)

        firmwareVersion @8 :Text;
        # Firmware version
    }

    struct Id {
        # HID-IO Command Id info

        uid @0 :UInt32;
        # Unique id of the HID-IO Command Id

        name @1 :Text;
        # Name of the HID-IO Command Id
    }


    cliCommand @0 (command :Text) -> ();
    # CLI command

    sleepMode @1 () -> (status :SleepModeStatus);
    # Attempt to have device go into a sleep state

    flashMode @2 () -> (status :FlashModeStatus);
    # Attempt to have the device enter flash mode

    manufacturingTest @3 (cmd :Manufacturing) -> (status :ManufacturingStatus);
    # Send a device specific manufacturing test command
    # Must have full auth-level to use

    info @4 () -> (info :Info);
    # Retrieves HID-IO information from the device

    supportedIds @5 () -> (ids :List(Id));
    # Lists the supported HID-IO command Ids by the device
    # Must have full auth-level to use

    test @6 (data :Data) -> (data :Data);
    # Send an arbitrary piece of data to device as a test command
    # Will Ack piece of data back if successful
    # Must have full auth-level to use

    pixelSetting @7 (command :PixelSetting) -> (status :PixelSettingStatus);
    # Configures LED settings
    # See pixelSet for updating specific LED channels

    pixelSet @8 (data :PixelSet) -> (status :PixelSetStatus);
    # Sets specific LED channels
}
