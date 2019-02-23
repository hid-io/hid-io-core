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

@0x89950abd85ed15de;

## Imports ##

using Common = import "common.capnp";



## Interfaces ##

interface HostMacro extends(Common.HIDIONode) {
    # Used by device to call host-side scripts
    struct Signal {
        union {
            macroNum :group {
                num @0 :UInt32;
            }
            macroName :group {
                name @1 :Text;
            }
        }
    }

    interface Commands {
    }
}

