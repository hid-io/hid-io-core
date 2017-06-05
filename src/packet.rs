/* Copyright (C) 2017 by Jacob Alexander
 *
 * This file is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This file is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this file.  If not, see <http://www.gnu.org/licenses/>.
 */

mod module;
mod device;


/// HostPackets originate from the host and are sent to the device
enum HostPacket {
}

/// DevicePackets originate from the device and are sent to the host
enum DevicePacket {
}

/// HostPacketMsgs originate from the host and are sent to the device
struct HostPacketMsg {
    packet : HostPacket,
    module : module::ModuleInfo,
    device : device::hidusb::hidapi::HidDeviceInfo,
}

/// DevicePacketMsgs originate from the device and are sent to the host
struct DevicePacketMsg {
    packet : DevicePacket,
    module : module::ModuleInfo,
    device : device::hidusb::hidapi::HidDeviceInfo,
}

struct PacketMsg2 {
    pid      : u32,        // Packet id
    plength  : u32,        // Packet data length
    ptype    : u8,         // enum? Packet type
    ppayload : Vec<u8>,    // Packet payload (entire payload, combines continued packets)
}


