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

@0xd59b9bf67dedf44d;

## Imports ##

using import "hid.capnp".HIDInfo;



## Structs ##

struct BLEMouseInfo {
    # This struct contains general information about a BLE mouse
    # These fields are mostly informational to allow whatever calling the API
    # to make better decisions about which devices are connected.

    hid @0 :HIDInfo;
    # hidapi information
}

