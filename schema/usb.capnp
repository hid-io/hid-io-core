# Copyright (C) 2019 by Jacob Alexander
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

@0xdc5bca6568b19313;

## Structs ##

struct USBInfo {
    # This struct contains general information about a USB device
    # (e.g. from libusb)

    struct Configuration {
    # This struct contains general information about a USB device configuration descriptor
    # (e.g. from libusb)

        struct InterfaceSetting {
        # This struct contains a list of interface settings
        # Each interface may specify a number of alternate interfaces (this is separate from Configurations
        # (e.g. from libusb)

            struct Interface {
                # This struct contains general information about a USB device interface descriptor
                # (e.g. from libusb)

                struct Endpoint {
                    # This struct contains general information about a USB device endpoint descriptor
                    # (e.g. from libusb)

                    enum Direction {
                        in @0;
                        out @1;
                    }

                    enum TransferType {
                        control @0;
                        isochronous @1;
                        bulk @2;
                        interrupt @3;
                    }

                    number @0 :UInt8;
                    # USB endpoint index

                    address @1 :UInt8;
                    # USB endpoint address

                    direction @2 :Direction;
                    # USB endpoint direction

                    transferType @3 :TransferType;
                    # USB endpoint transfer type

                    maxPacketSize @4 :UInt16;
                    # USB endpoint maximum transfer size

                    interval @5 :UInt8;
                    # USB polling interval
                }

                interfaceNumber @0 :UInt8;
                # USB interface index

                settingNumber @1 :UInt8;
                # USB alternate setting number

                classCode @2 :UInt8;
                # USB interface class code

                subClassCode @3 :UInt8;
                # USB interface sub-class code

                protocolCode @4 :UInt8;
                # USB interface protocol code

                interfaceName @5 :Text;
                # Interface name
                # Might not be set due to:
                # - Partial Windows drivers
                # - Not set by manufacturer

                numEndpoints @6 :UInt8;
                # Number of endpoints for this interface

                endpoints @7 :List(Endpoint);
                # List of endpoints
                # This may not be a direct index as some keyboards may skip endpoint indices
            }

            number @0: UInt8;
            # USB interface setting index

            interfaces @1 :List(Interface);
        }

        number @0 :UInt8;
        # USB configuration index

        maxPower @1 :UInt16;
        # Maximum power in mA

        selfPowered @2 :Bool;
        # Whether device is self-powered or not

        remoteWakeup @3 :Bool;
        # Wheter device supports remote wakeup or not

        configurationName @4 :Text;
        # Name of the configuration
        # Might not be set due to:
        # - Partial Windows drivers
        # - Not set by manufacturer

        numInterfaces @5 :UInt8;
        # Number of interfaces for this configuration

        interfaceSettings @6 :List(InterfaceSetting);
        # List of interfaces
        # This may not be a direct index as some keyboards may skip interface indices
    }

    enum Speed {
        unknown @0;
        low @1;
        full @2;
        high @3;
        super @4;
    }

    busNumber @0 :UInt8;

    address @1 :UInt8;

    speed @2 :Speed;
    # Speed of the USB device

    usbVersion @3 :UInt16;
    # USB version of the device

    deviceVersion @4 :UInt16;
    # Device version/revision

    manufacturerString @5 :Text;
    # Manufacturers string
    # Might not be set due to:
    # - Partial Windows drivers
    # - Not set by manufacturer

    productString @6 :Text;
    # Product string
    # Might not be set due to:
    # - Partial Windows drivers
    # - Not set by manufacturer

    serialNumber @7 :Text;
    # Serial number string
    # Might not be set due to:
    # - Partial Windows drivers
    # - Not set by manufacturer

    classCode @8 :UInt8;
    # Main class code (not usually set for hid devices at this level)

    subClassCode @9 :UInt8;
    # Sub class code (not usually set for hid devices at this level)

    protocolCode @10 :UInt8;
    # Protocol code (not usually set for hid devices at this level)

    vendorId @11 :UInt16;
    # USB-IF assigned vendor id

    productId @12 :UInt16;
    # Vendor assigned product id

    maxPacketSize @13 :UInt8;
    # Max packet size for endpoint 0 (Control)

    activeConfiguration @14 :UInt8;
    # Active configuration

    numConfigurations @15 :UInt8;
    # Total number of configurations

    configurations @16 :List(Configuration);
    # List of configurations
    # This may not be a direct index as some keyboards may skip configuration indices
}

