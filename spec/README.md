# HID-IO Spec

Unlike the KLL spec which puts some very heavy processing/resource requirements on the MCU, the goal HID-IO is to fit on the smallest of keyboard/input device firmwares while still maintaining very high OS compatibility. Virtually all the commands will be optional, and the supported commands are defined by the keyboard at runtime.


### Modifications
* 2016-02-14 - Initial Draft (HaaTa)
* 2017-07-XX - Initial Implementation - v0.1.0 (HaaTa)

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

When USB was developed in the mid-90s, one of the goals was to expand the possibilities of external devices and still use a signal interface. And to this end, they succeeded. We no longer have PS/2 ports for keyboards, game ports, parallel ports and serial ports (still around, but not nearly as pervasive) because USB can take care of all these different interfaces even all of them over the same cable. Without going into too much detail, USB defines descriptors programmed into the devices that specifies what sort of data will be transmitted/received and what it will look like. The various USB specs go into great detail about all the variations possible and which ones require drivers and which ones should have host side drivers built-in (e.g. USB HID).

However, the problem comes when you want to add something that is not supported within the USB HID spec. In order to propose an addition you need to be a member of USB-IF ($$$$) and get your changes approved by the committee. Then on top of that you need to either implement the driver or wait until the relevant OSs and devices support the USB HID additions. And to top it all off, still won't work on older computers. Needless to say, the effort/time:reward is pretty low when you're looking to extend the capabilities of a keyboard beyond what the USB HID spec defines them to be.

Instead, the spec proposes a very simple sideband protocol that input devices can use **and** extend with a relatively quick approval process without requiring any sort of financial remuneration (i.e. it's free).


## Interface

**Note**: Bluetooth and other interfaces are TBD until a full OS compatibility evaluation is done.

In order to communicate between the host and the device an interface is needed.

On USB this means some sort of descriptor is required to open up a communication pipe. Ideally, we would be able to use a special provision in HID called Raw HID. Raw HID is just a couple USB interrupt endpoints (Tx and Rx) that supports sending fixed sized data packets. In terms of data rate, minimal descriptor and code size, this would be an ideal interface to use. On Windows Raw HID is even driver-less. Unfortunately, this is not the case for Linux and Mac, which (uncharacteristically for HID) require drivers or releasing the HID devices from other HID drivers. In some cases there is not a way to selectively release single endpoints from the driver which may render your keyboard useless. For the most part, this means, that Raw HID is not really a good fit for HID-IO.

A good (and similar) alternative is called a Vendor Specific interface which I'll refer from now on as Raw IO. Raw IO defines two interrupt transfer endpoints, Rx and Tx, and a given max packet size. This is very similar to Raw HID, but has the added bonus of not requiring a driver on Mac or Linux. Windows requires a WinUSB/libusb shim driver (what the zadig (http://zadig.akeo.ie/) loader is for) unfortunately, but no reboot is required and this part can be automated due to the client side software also needing to be installed. Utilizing libusb (or similar) generic drivers the client side code base can be dramatically simplified. The max packet size may be set to whatever is ideal for the input device and still adheres to the USB spec. For Full-speed USB 2.0 this is a maximum of 64 bytes.

```
|Interrupt Max Packet Sizes|
Low Speed - 8 byte
Full Speed - 64 bytes or less
Hi-Speed - 1024 bytes or less (this also applies to 3.1 Gen 1 and 2)
```

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

The length field is the a value between 1 and <max packet size> - 2. In addition to the dedicated byte, this field has 3 additional bits from the Header byte which are the MSBs. This 10 bit number (max 1023) is sufficiently large to handle any interrupt max packet length as defined by the USB spec (as of writing). It is the responsibility of the sender to make sure the length value does not exceed the max packet size as USB does not have automatic chunking available for this type of interface.

All payloads are Id specific and may include any sort of day without restriction as long as it fits within the max payload size. If the payload is larger, the payload may be chunked into Continued packets. The receiving side will need to keep track of the previous packet type to process the continued packet.

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

Supported Ids
```
0x00

Requests a list of supported Ids on the device. This includes required Ids. Use the header byte to determine the Id width.

+> 0x00 0x01 (Ids 0 and 1)
-> (No payload on error)
```

Get Info
```
0x01 <property>

Requests a property from the device.

0x00 - HID-IO Major Version (16 bit)
0x01 - HID-IO Minor Version (16 bit)
0x02 - HID-IO Minor Version (16 bit)
0x03 - Device Name (ascii)

+> <property>
-> <invalid property value>
```


**Host Required Commands**

Supported Ids
```
0x00

Requests a list of supported Ids on the host. This includes required Ids. Use the header byte to determine the Id width.

+> 0x00 0x01 (Ids 0 and 1)
-> (No payload on error)
```

Get Info
```
0x01 <property>

Requests a property from the host.

0x00 - HID-IO Major Version (16 bit)
0x01 - HID-IO Minor Version (16 bit)
0x02 - HID-IO Minor Version (16 bit)
0x03 - OS Type (8 bit)
 * 0x00 - Unknown
 * 0x01 - Windows
 * 0x02 - Linux
 * 0x03 - Android
 * 0x04 - Mac
 * 0x05 - iOS
0x04 - OS Verson (ascii)
0x05 - Host software name

+> <property>
-> <invalid property value>
```


**Device Optional Commands**

Send UTF-8 character stream
```
0x10 <utf-8 characters>

Sends the given list of UTF-8 characters, these will be printed out to the current keyboard focus in order on the host OS.

+> (No payload)
-> (No payload)
```

Trigger Host Side Macro
```
0x11 <macro id number> <macro id number 2>...

Triggers a given host side macro using id numbers.

+> (No payload)
-> (macro ids that were not successful/do not exist)
```

Keyboard Scan Code Id State
```
0x12 <Scan Codes currently pressed>

List of scan codes currently pressed. Should be updated whenever the USB buffer would be updated. This command is used along with the host side Keyboard Layout command to translate Scan Codes to USB codes.
WARNING: Should be limited to function keys to protect from keyloggers. But it would be possible to use this as host side macro processing if the user gives consent.

+> (No payload)
-> (No payload)
```


**Host Optional Commands**

Get Device Properties
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

USB Key State
```
0x11 <mode> <usb code> [<usb code>...]

Sends a list of USB key codes to activate or release on the keyboard.
Modes:
 * 0x00 - Set following keys as pressed
 * 0x01 - Release following keys

+> (No payload)
-> (Mode and keys which the state could not be changed).
```

Keyboard Layout
```
0x12 <layer>

Returns a list of Scan Code:USB Code mappings for each key on the given layer. 0 is considered the default state of the keyboard. To request all layers, request until an nak is received for the command.
Width of scan code is the number of bytes. In general this will be 0x01, and in extreme cases for a keyboard with over 256 keys, 0x02.

+> <width of scan code> <scan code> <usb code> [<scan code> <usb code>...]
-> No payload, layer doesn't exist
```

Button Layout
```
0x13

Returns the physical properties of how the buttons are laid out and what type of keycap is on each.

TODO, finalize how the data will be sent and what units. Requires: x,y,z, rx,ry,rz per scan code.

+> <width of scan code> TODO
-> (No payload on error)
```

Keycap Types
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

LED Layout
```
0x15

Returns the physical properties of how the LEDs are laid out on the device. Each LED is given a unique id number defined by the device.

TODO, finalize how the data will be sent and what units. Requires: x,y,z, rx,ry,rz per scan code.

+> <width of led id> TODO
-> (No payload on error)
```

Flash Mode
```
0x16

Puts the keyboard into flash mode if remote re-flashing is enabled on the keyboard.
WARNING: Do not allow the keyboard to enter flash mode remotely by default. This is a serious security hazard.

+> No response, the HID-IO interface will disappear
-> Error code
 * 0x00 - Not supported
 * 0x01 - Disabled
```



## ID List

* 0x00 - Supported Ids
* 0x01 - Get Info
* 0x02 - Test Packet
* 0x03 - Reset HID-IO
* 0x04..0x0F - **Reserved**
* 0x10 - Get Device Properties
* 0x11 - USB Key State
* 0x12 - Unicode Key State
* 0x13 - Keyboard Layout
* 0x14 - Key Layout
* 0x15 - Key Shapes
* 0x16 - LED Layout
* 0x17 - Flash Mode




