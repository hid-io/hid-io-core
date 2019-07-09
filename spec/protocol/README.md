# HID-IO Protocol Spec

Unlike the KLL spec which puts some very heavy processing/resource requirements on the MCU, the goal HID-IO is to fit on the smallest of keyboard/input device firmwares while still maintaining very high OS compatibility. Virtually all the commands will be optional, and the supported commands are defined by the keyboard at runtime.


### Modifications
* 2016-02-14 - Initial Draft (HaaTa)
* 2017-07-15 - Proposed Implementation - v0.1.0 (HaaTa)
* 2019-01-07 - Updated Proposed Implementation - v0.1.1 (HaaTa)
* 2019-02-09 - Clarification of continued packet format - v0.1.2 (HaaTa)
* 2019-02-15 - Adding pixel control HID-IO packets - v0.1.3 (HaaTa)

## Glossary

* Big-endian - Byte number ordering such that the most significant bits are at the end of the byte range
* Endpoint - USB term for a place to send data
* HID - Human Interface Device
* Interface - Connection between two entities. In USB this is a logical grouping of endpoints.
* Little-endian - Byte number ordering such that the most significant bits are at the beginning of the byte range
* MCU - Microcontroller Unit


## Summary

HID-IO is an input device sideband protocol specification designed to give additional functionality and control to input devices beyond what is available using HID interfaces (e.g. USB, Bluetooth). The intent is to have another sort of driver/program/daemon run on the host computer which can communicate directly with the keyboard over the given interface. Some notable features this will enable would be to output UTF-8 symbols, change-locales on the fly, host side defined macros and on-screen keyboard layout views. Special consideration has been given to make sure that HID-IO will not be turned into a sort of keylogger interface without the user being aware.

Each command is prefaced with an Id. All Ids are reserved, this means that a firmware designer would need to request Ids for custom commands if the standard set of commands are not sufficient. The possible number of Ids is large enough such that it should not need to be extended, but allows for small packet sizes.

The layout of the protocol, and designated interface(s) have been chosen to simplify firmware design so it may fit in smaller MCUs. Nearly all Ids are optional which allows the firmware designer to pick and choose the relevant ones for their particular keyboard or save precious flash and ram on the MCU.

This is not an ultra-low latency interface, it's meant to complement the USB HID interrupt endpoints.


## Scope

When USB was developed in the mid-90s, one of the goals was to expand the possibilities of external devices and still use a signal interface. And to this end, they succeeded. We no longer have PS/2 ports (for keyboards and mice), game ports, parallel ports and serial ports (still around, but not nearly as pervasive) because USB can take care of all these different interfaces even all of them over the same cable. Without going into too much detail, USB defines descriptors programmed into the devices that specifies what sort of data will be transmitted/received and what it will look like. The various USB specs go into great detail about all the variations possible and which ones require drivers and which ones should have host side drivers built-in (e.g. USB HID).

However, the problem comes when you want to add something that is not supported within the USB HID spec. In order to propose an addition you need to be a member of USB-IF ($$$$) and get your changes approved by the committee. Then on top of that you need to either implement the driver or wait until the relevant OSs and devices support the USB HID additions. And to top it all off, the new USB HID spec still won't work on older computers and devices. Needless to say, the effort/time:reward is pretty low when you're looking to extend the capabilities of a keyboard beyond what the USB HID spec defines them to be.

Instead, the spec proposes a very simple sideband protocol that input devices can use **and** extend with a relatively quick approval process without requiring any sort of financial remuneration (i.e. it's free).


## Interface

**Note**: Bluetooth and other interfaces are TBD until a full OS compatibility evaluation is done.

**Note2**: USB Raw HID is currently being evaluated to determine if it will suit hid-io better going into the future. Raw HID is supported on Bluetooth which is partially the reason to move away from a USB Vendor Specific Interface.

In order to communicate between the host and the device an interface is needed.

On USB this means some sort of descriptor is required to open up a communication pipe. Ideally, we would be able to use a special provision in HID called Raw HID. Raw HID is just a couple USB interrupt endpoints (Tx and Rx) that supports sending fixed sized data packets. In terms of data rate, minimal descriptor and code size, this would be an ideal interface to use. On Windows Raw HID is even driver-less. For Linux hidapi (https://github.com/signal11/hidapi) exists, and while it's not well maintained anymore it should provide a good interface to hidraw. On macOS hidapi uses IOHidManager which may work well, but since macOS changes the driver stack often it may be necessary to maintain a macOS driver.

```
|Interrupt Max Packet Sizes|
Low Speed - 8 byte
Full Speed - 64 bytes or less
Hi-Speed - 1024 bytes or less (this also applies to 3.1 Gen 1 and 2)
```


### Raw HID

Since Full Speed USB 2.0 is the most common usage on keyboards, we shall start with 64 byte packets.
Going to larger packet sizes is rather easy, but going too large would slow down the bus and would really only be necessary if lots of data were being transmitted.
If it's found that a lot of data needs to be transmitted (and USB 2.0 high-speed is available), then the packet size can be re-visited or a 2nd interface can be created for large transmission sizes.

In order to identify that a specific device interface is available to client side software. We'll be using three interface descriptor fields.

* Usage Page (0xFF1C) - Read as FFK
* Usage (0x1100) - Read as II 0

PJRC recommends that the Usage Page be between 0xFF00 and 0xFFFF and the Usage be between 0x0100 and 0xFFFF (https://github.com/PaulStoffregen/cores/blob/master/teensy3/usb_desc.h#L531).
The Usage Pages (https://www.usb.org/sites/default/files/documents/hut1_12v2.pdf Section 3) 0xFF00 to 0xFFFF are deemed as Vendor-defined and are safe to use.
It is still not apparent why the Usage should be greater than 0x0100, but it is likely for OS compatibility reasons.


### Vendor Specific (deprecated)

**Note:** The reason for not using a Vendor Specific interface is to simplify Windows driver integration (which is painful). And make sure that there is a minimum permissions level such that random applications cannot attach to the HID-IO interface and build keyloggers directly.

A good (and similar) alternative is called a Vendor Specific interface which I'll refer from now on as Raw IO. Raw IO defines two interrupt transfer endpoints, Rx and Tx, and a given max packet size. This is very similar to Raw HID, but has the added bonus of not requiring a driver on Mac or Linux. Windows requires a WinUSB/libusb shim driver (what the zadig (http://zadig.akeo.ie/) loader is for) unfortunately, but no reboot is required and this part can be automated due to the client side software also needing to be installed. Utilizing libusb (or similar) generic drivers the client side code base can be dramatically simplified. The max packet size may be set to whatever is ideal for the input device and still adheres to the USB spec. For Full-speed USB 2.0 this is a maximum of 64 bytes.

In order to identify that a specific device interface is available to client side software. We'll be using three interface descriptor fields.

* bInterfaceClass (0xFF)
* bInterfaceSubClass (0x49)
* bInterfaceProtocol (0x4F)

According to the USB spec, to use the Vendor Specific/Raw IO interface **bInterfaceClass** must be set to [0xFF](http://www.usb.org/developers/defined_class/#BaseClassFFh). While both **bInterfaceSubClass** and **bInterfaceProtocol** may be set to any value. Most Vendor Specific interfaces set all 3 values to 0xFF, so to differentiate against these devices HID-IO requires 0x49 (ASCII I) and 0x4F (ASCII O) be set.
Any versioning will be done using the spec protocol, so it shouldn't be necessary to add any more subclass and protocol values.

For more reliable host side USB device keying, it is recommended that the host program to locate at least 1 USB HID interface first, before looking for the Raw IO interface.


## Protocol

The idea behind this protocol is to simplify device side processing and not imposing a command limit. This is to encourage its use on a wide variety of input devices. The underlying interface (e.g. USB) takes care of error detection and packet re-transmission so there are no additional provisions for detecting errors (would increase the load on the MCU). For this reason it is not recommended to use the HID-IO protocol on wire level interfaces such as UART/RS-232, I2C and SPI without additional error correction/re-transmission packet framing.

__Requirements of transmission__
* One packet per medium packet (e.g. USB), no packet bundling. This makes synchronization easy if either side gets into a bad state.
* Every data packet must be responded by an Ack or Nak packet.
* Each side periodically (~1-5 seconds) sends Sync packets as a keep-alive. If more than one Sync is received while waiting for an Ack or Nak the previous data packet was not successfully processed. Sync packets should only be sent if the device/host has not sent a packet during a given time interval.
* If a request is received while sending its own query, finish sending the query, then immediately process the request. Do not send a Sync between unless sending a packet using the max payload. In this case a Sync packet should be sent immediately after to tell the receiver that the packet will not be continued.
* When receiving a Nak packet, any pending continued packets for that sequence must be dropped. Nak packet may, or may not contain a payload. No payload indicates that the request is not suported.


**Packet Types**

HID-IO has a 5 different packet types: Data **b000**, Acknowledge **b001**, Negative Acknowledge **b002**, Sync **b003** and Continued **b004** packets. Packets labels 5 through 7 are reserved. These 3 bits are included in the Header byte of each packet.

```
VVVW XYZZ
VVV - Packet type
  W - Continued
  X - Id width
  Y - Reserved
 ZZ - Upper length bits

0110 0000 (0x60) - Sync packet

|Packet Types|
b000 - Data
b001 - Acknowledge
b010 - Negative Acknowledge
b011 - Sync
b100 - Continued
b101 to b111 are Reserved

|Continued|
b0 - All data payload fits into one packet
b1 - An additional continued packet is necessary
(last packet sets to b0 to indicate complete)

|Id Widths|
b0 - 16 bit
b1 - 32 bit

|Upper Length Bits|
b11 1111 1111 - 1023
b00 0000 0001 - 1
```

Except for the Sync packet, which only requires a single byte transmission, the rest of the packets require at least two more pieces of information: Length and Id. Length is the number of payload bytes left in the packet while Id is the unique identifier to specify each command/response. The length byte is always after the header byte.

The length field is the a value between 1 and <max packet size> - 2. In addition to the dedicated byte, this field has 3 additional bits from the Header byte which are the MSBs. This 10 bit number (max 1023) is sufficiently large to handle any interrupt max packet length as defined by the USB spec (as of writing). It is the responsibility of the sender to make sure the length value does not exceed the max packet size as USB does not have automatic chunking available for this type of interface. When the continued bit is set (W=1), then the length field represents the number of packets that are pending for the continued packet. A packet with W=1 always contains a maximum payload.

All payloads are Id specific and may include any sort of data without restriction as long as it fits within the max payload size. If the payload is larger, the payload may be chunked into Continued packets. The receiving side will need to keep track of the previous packet type to process the continued packet.

The Id Width field indicates whether the Id is 16 bits or 32 bits wide. As long as the Id is lower than 2^16, a 16 bit field is always supported. Only use 32 bit Ids when required, not all firmwares will support 32 bit Ids.

__Data Packet__
```
<data> <length> <Id> [payload]

Data Packet, 32 bit Id, Continued, 4 byte length (actual length 6), Id 15
0x18 0x04 0x0F 0x00 0x00 0x00
```

__Acknowledge Packet__
```
<ack> <length> <Id> [payload]

Ack packet, 16 bit id, 3 byte length (actual length 5), Id 1025, Payload 0x32
0x20 0x03 0x01 0x04 0x32
```

__Negative Acknowledge Packet__
```
<nak> <length> <Id> [payload]

Nak packet, 16 bit id, 260 byte length (actual length 262), Id 40, Payload starts from ...
0x40 0x04 0x28 0x00 ...
```

__Continued Packet__
```
<cont> <length> <Id> [payload]

Continued packet, 16 bit id, 2 length (actual length 4), Id 10, Payload 0xFE
0x80 0x02 0x0A 0x00 0xFE
```

__Sync Packet__
```
<sync>

Sync Packet
0x60
```


## IDs

Each HID-IO command has a unique Id associated with it. To future-proof, HID-IO supports up to 32 bit Id values.

While most of the Ids are optionally implemented, there are a few that are required so that both the device and the host can gather information about the other.

The next sections will use the following format.

\<Name>
```
<Id> [payload]

<description>

+> <successful payload example>
-> <negative payload example>
```


**Device Required Commands**

#### Supported Ids
```
0x00

Requests a list of supported Ids on the device. This includes required Ids. Use the header byte to determine the Id width.

+> 0x00 0x01 (Ids 0 and 1)
-> (No payload on error)
```

#### Get Info
```
0x01 <property>

Requests a property from the device.

0x00 - HID-IO Major Version (16 bit)
0x01 - HID-IO Minor Version (16 bit)
0x02 - HID-IO Patch Version (16 bit)
0x03 - Device Name (ascii)
0x04 - Device Serial Number (ascii)
0x05 - Device Version (ascii)
0x06 - Device MCU (ascii) (e.g. mk20dx256vlh7, atsam4s8b, atmega32u4)
0x07 - Firmware Name (ascii) (e.g. kiibohd, QMK, etc.)
0x08 - Firmware Version (ascii)

+> <property>
-> <invalid property value>
```


**Host Required Commands**

#### Supported Ids
```
0x00

Requests a list of supported Ids on the host. This includes required Ids. Use the header byte to determine the Id width.

+> 0x00 0x01 (Ids 0 and 1)
-> (No payload on error)
```

#### Get Info
```
0x01 <property>

Requests a property from the host.

0x00 - HID-IO Major Version (16 bit)
0x01 - HID-IO Minor Version (16 bit)
0x02 - HID-IO Patch Version (16 bit)
0x03 - OS Type (8 bit)
 * 0x00 - Unknown
 * 0x01 - Windows
 * 0x02 - Linux
 * 0x03 - Android
 * 0x04 - macOS
 * 0x05 - iOS
 * 0x06 - ChromeOS
0x04 - OS Version (ascii)
0x05 - Host software name

+> <property>
-> <invalid property value>
```


**Device Optional Commands**

#### UTF-8 character stream
```
0x17 <utf-8 characters>

Sends the given list of UTF-8 characters, these will be printed out to the current keyboard focus in order on the host OS.

+> (No payload)
-> (No payload)
```

#### UTF-8 state
```
0x18 <utf-8 characters>

Sends a utf-8 state list.
All of the characters sent will be held.
To release a character send this packet again without that particular symbol.

+> (No payload)
-> (No payload)
```

#### Trigger Host Macro
```
0x19 <macro id number> <macro id number 2>...

Triggers a given host side macro using id numbers (16-bit).

+> (No payload)
-> (macro ids that were not successful/do not exist)
```

#### KLL Trigger State
```
0x20 <trigger type1:8 bit> <trigger id1:8 bit> <trigger state1:8 bit> <trigger type2:8 bit> <trigger id2:8 bit> <trigger state2:8 bit>...

List of trigger ids activated at the start of a macro processing cycle.
Each trigger is represented by a 3-tuple of 8-bit values.
Using triggers alone you cannot deduce which key was pressed.
However using the scancode to USB mapping it is possible to determine which keys were pressed.

+> (No payload)
-> (No payload)
```


**Host Optional Commands**

#### Get Properties
```
0x10 <command> [<field id>]

Gets an arbitrary ascii property from the device. Can be used for additional (and perhaps dynamic) data fields.
Commands:
 * 0x00 - List 8 bit field Ids
 * 0x01 - Get the name of the field (ascii)
 * 0x02 - Get the value in the field (ascii)

+> Either a list of 8 bit field Ids or an ascii string with the requested value
-> (Command and possible filed id of the failed command)
```

#### USB Key State
```
0x11 <mode> <usb code> [<usb code>...]

Sends a list of USB key codes to activate or release on the keyboard.
Modes:
 * 0x00 - Set following keys as pressed
 * 0x01 - Release following keys

+> (No payload)
-> (Mode and keys which the state could not be changed).
```

#### Keyboard Layout
```
0x12 <layer>

Returns a list of Scan Code:USB Code mappings for each key on the given layer. 0 is considered the default state of the keyboard. To request all layers, request until an nak is received for the command.
Width of scan code is the number of bytes. In general this will be 0x01, and in extreme cases for a keyboard with over 256 keys, 0x02.

USB Code Types:
 * 0x00 - USB Keyboard
 * 0x01 - LED
 * 0x02 - Consumer Ctrl Space 0x00
 * 0x03 - Consumer Ctrl Space 0x01
 * 0x04 - Consumer Ctrl Space 0x02
 * 0x05 - System Ctrl
 * TBD (Mouse/Joystick)

+> <width of scan code> <scan code> <usb code type> <usb code> [<scan code> <usb code>...]
-> No payload, layer doesn't exist
```

#### Button Layout
```
0x13

Returns the physical properties of how the buttons are laid out and what type of keycap is on each.

+>  <id:16 bit> <x:float> <y:float> <z:float> <rx:float> <ry:float> <rz:float> <id2:16 bit>...
-> (No payload on error)
```

#### Keycap Types
```
0x14 <command> [option]

Used to determine the types of keycap shapes the keyboard uses.
Command:
* 0x00 - List keycap ids
* 0x01 - Query type of keycap

TODO - Need a good way of physically representing all types of keycaps, including ISO and L-Enters.

+> TODO
-> TODO
```

#### LED Layout
```
0x15 <type>

Type:
* 0x00 - Position - <id:16 bit> <x:float> <y:float> <z:float> <rx:float> <ry:float> <rz:float> <id2:16 bit>...
  Returns the physical properties of how the LEDs are laid out on the device.
  Each LED is given a unique id number defined by the device.
* 0x01 - Grid - <height:16 bit> <width:16 bit> <id:16 bit> <id2:16 bit>...
  Returns a list of unique ids such that they are organized in a grid.
  Any ids set to 0 are empty spaces within the grid and can safely be ignored.
* 0x02 - List - <id:16 bit> <id2:16 bit>...
  Returns a list of unique LED ids.
* 0x03 - Scan Code Mapping - <scan code:16 bit> <id:16 bit> <scan code2:16 bit> <id2:16 bit>...
  Returns a list of scancode to LED id mappings.

If a type is not supported an error is returned (no payload).

+> See payloads per type.
-> (No payload on error)
```

#### Flash Mode
```
0x16

Returns the scancode that must be pressed on the physical keyboard before flash mode will activate.
Once the scancode is pressed the device will enter flash mode and the interface will disappear.
If any other scancode is pressed the action will be cancelled.

WARNING: Do not allow flash mode without some sort of physical interaction as this is a serious security hazard.

+> <scancode:16 bit>
-> Error code
 * 0x00 - Not supported
 * 0x01 - Disabled
```

#### Pixel Setting
```
0x21 <command:16 bits> <argument:16 bits>

Controls various LED modes on the device.
Mainly used to put the LED controller(s) into the correct state
 * 0x0001 - HID-IO LED control
            Enable/Disable LED control from HID-IO
            The device should not update any LEDs when enabled
            Args:
            * 0x0000 - Disable
            * 0x0001 - Enable, full speed
            * 0x0002 - Enable, frame wait (waits for Next frame 0x0004 before the device updates the LEDs)
 * 0x0002 - Reset LED controller
            This should re-initialize the LED controller.
            Often useful when I2C devices get into a bad state.
            * 0x0000 - Soft reset - clear buffers
            * 0x0001 - Hard reset - reset hardware bus (if exists)
 * 0x0003 - Clear LEDs
            Sets device side pixel states to off (all LEDs should be off).
            If HID-IO LED control is on, LEDs will stay off.
            If HID-IO LED control is off, LEDs may turn back on.
            Args:
            * 0x0000 - Clear
 * 0x0004 - Next frame
            If frame wait control mode is enable, tell the device to update the LEDs and allow writing to the next buffer.
            If the device hasn't finished writing to the LEDs will NAK and must be resent.
            Args:
            * 0x0000 - 1 frame

+> (No payload)
-> (No payload)
```

#### Pixel Set (1 ch, 8 bit)
```
0x22 <starting pixel address:16 bits> <pixel1 ch1:8 bits> <pixel2 ch1:8 bits>...

Starting from the given pixel address, set the first channel using an 8 bit value.
If the pixel is is a smaller size internally ignore any value greater than the internal size (do not send a NAK).
For example:
 47  on a 1 bit display should be ignored
 200 on a 7 bit display should be ignored

If there is no channel for a given pixel (pixel address is unassigned), ignore (do not send a NAK).

+> (No payload)
-> (No payload)
```

#### Pixel Set (3 ch, 8 bit)
```
0x23 <starting pixel address:16 bits> <pixel1 ch1:8 bits> <pixel1 ch2:8 bits> <pixel1 ch3:8 bits> <pixel2 ch1:8 bits>...

Starting from the given pixel address, set the first channel using an 8 bit value.
If the pixel is is a smaller size internally ignore any value greater than the internal size (do not send a NAK).
For example:
 47  on a 1 bit display should be ignored
 200 on a 7 bit display should be ignored

If there is no channel for a given pixel (pixel address is unassigned or using on a 1 channel pixel), ignore (do not send a NAK).

+> (No payload)
-> (No payload)
```

#### Pixel Set (1 ch, 16 bit)
```
0x24 <starting pixel address:16 bits> <pixel1 ch1:16 bits> <pixel2 ch1:16 bits>...

Starting from the given pixel address, set the first channel using a 16 bit value.
If the pixel is is a smaller size internally ignore any value greater than the internal size (do not send a NAK).
For example:
 47   on a  1 bit display should be ignored
 200  on a  7 bit display should be ignored
 2000 on an 8 bit display should be ignored

If there is no channel for a given pixel (pixel address is unassigned), ignore (do not send a NAK).

+> (No payload)
-> (No payload)
```

#### Pixel Set (3 ch, 16 bit)
```
0x25 <starting pixel address:16 bits> <pixel1 ch1:16 bits> <pixel1 ch2:16 bits> <pixel1 ch3:16 bits> <pixel2 ch1:16 bits>...

Starting from the given pixel address, set the first channel using a 16 bit value.
If the pixel is is a smaller size internally ignore any value greater than the internal size (do not send a NAK).
For example:
 47   on a  1 bit display should be ignored
 200  on a  7 bit display should be ignored
 2000 on an 8 bit display should be ignored

If there is no channel for a given pixel (pixel address is unassigned or using on a 1 channel pixel), ignore (do not send a NAK).

+> (No payload)
-> (No payload)
```


### Test, with
## ID List

* 0x00 - (Host/Device) [Supported Ids](#supported-ids)
* 0x01 - (Host/Device) [Get Info](#get-info)
* 0x02 - (Host/Device) [Test Packet](#test-packet)
* 0x03 - (Host/Device) [Reset HID-IO](#reset-hid-io)
* 0x04..0x0F - **Reserved**
* 0x10 - (Host)        [Get Properties](#get-properties)
* 0x11 - (Host)        [USB Key State](#usb-key-state)
* 0x12 - (Host)        [Keyboard Layout](#keyboard-layout)
* 0x13 - (Host)        [Button Layout](#button-layout)
* 0x14 - (Host)        [Keycap Types](#keycap-types)
* 0x15 - (Host)        [LED Layout](#led-layout)
* 0x16 - (Host)        [Flash Mode](#flash-mode)
* 0x17 - (Device)      [UTF-8 Character Stream](#utf-8-character-stream)
* 0x18 - (Device)      [UTF-8 State](#utf-8-state)
* 0x19 - (Device)      [Trigger Host Macro](trigger-host-macro)
* 0x20 - (Device)      [KLL Trigger State](#kll-trigger-state)
* 0x21 - (Host)        [Pixel Setting](#pixel-setting)
* 0x22 - (Host)        [Pixel Set (1 ch, 8 bit)](#pixel-set-1-ch-8-bit)
* 0x23 - (Host)        [Pixel Set (3 ch, 8 bit)](#pixel-set-3-ch-8-bit)
* 0x24 - (Host)        [Pixel Set (1 ch, 16 bit)](#pixel-set-1-ch-16-bit)
* 0x25 - (Host)        [Pixel Set (3 ch, 16 bit)](#pixel-set-3-ch-16-bit)

