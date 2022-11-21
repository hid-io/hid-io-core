# hid-io-client

HID-IO Client Side application interface

The purpose of this crate is to provide a common set of functions that can be used to connect directly to hid-io-core over an FFI interface.
Please see [hid-io-client](../hid-io-client) if you are looking for a rust native library.

## Connecting to hid-io-core

TODO
- Point to docs on how to start hid-io-core
- How to connect to hid-io-core
- How to interact directly with hid-io-core
  * How to survive hid-io-core restarts
	* Get list of devices
- How to connect/interact with device
  * How to interact with multiple devices
	* How to survive device re-connects
	* Running manufacturing tests
	* Controlling LEDs
	* Interfacing with kll events
	* Manufacturing (initiate + watch)
- Easy way to expose/use capnproto api (without having to implement functions for each?)
  * FFI functions will need a set list of functions
	  + List of devices
		+ Restarts
		+ Connect to device
		+ Device re-connects
		+ Controlling LEDs
		+ Adjusting settings
		  - Activation point
- Replace large port of examples with hid-io-client
