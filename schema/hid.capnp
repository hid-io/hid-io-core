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

@0xa96fa0306d6044ce;

## Structs ##

struct HIDInfo {
    # TODO (HaaTa): Have way to query HID descriptor
    # This struct contains genreal information about a HID device
    # (e.g. from HIDAPI)

    path @0 :Text;

    vendorId @1 :UInt16;
    # USB-IF assigned vendor id

    productId @2 :UInt16;
    # Vendor assigned product id

    serial @3 :Text;
    # Device serial number

    releaseNumber @4 :UInt16;
    # Device/firmware release number

    manufacturerString @5 :Text;
    # Manufacturer name

    productString @6 :Text;
    # Product name

    usagePage @7 :UInt16;
    # HID Usage Page number

    usage @8 :UInt16;
    # HID Usage number

    interfaceNumber @9 :Int32;
    # HID Interface number

    isHIDIO @10 :Bool;
    # Compatible with HID-IO (checks usage_page and usage)
}

