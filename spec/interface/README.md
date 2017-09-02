# HID-IO Interface Spec

TODO


# HID-IO Interface Outline/Requirements

* Ability to register many different external programs/scripts at the same time
  + Each external script should not talk to another one through HID-IO
  + External scripts should not interfere with existing HID-IO protocol communications with devices
* Support multiple devices at the same time (2 keyboards, keyboard + mouse, etc.)
* Direct HID-IO protocol communication for some IDs
  + Three levels of access
    - Root/secure access (possibly key-presses?)
    - Userspace access (specific macro key was triggered, call animation, etc.)
    - Debug root access (directly communicate using HID-IO protocol with no restrictions, not enabled by default)
  + Three levels of control
    - Listen (wait for a specific macro event, can be listened by multiple scripts)
    - Request (request information from a keyboard, information is only sent back to requester)
    - Snoop (debug ability to monitor per Id request, not enabled by default)
* Support commands for requesting information from the HID-IO daemon (not necessarily just a pass-through to the HID device)


## Requirements for RPC/Scripting Interface

* Supports a wide variety of languages, preferably scripting
  + Python is a good starting point
* Relatively straightforward to write a script to listen or request data from a keyboard
* Cross-platform
* Uses sockets
  + 7185 (0x1c11) - Not currently assigned to anything
  + Only allows connections from localhost (127.0.0.1)
* Authentication
  + Thoughts? (not really settled on doing it this way, but some sort of authentication is needed for more sensitive operations)
  + Request a run-time key from the HID-IO daemon
    - hid-io --userspace-key
    - hid-io --secure-key # Requires root priviledges
  + Should use some sort of TLS for traffic
    - Key would be used to establish the tunnel


## Authentication

Thoughts about authentication.

For the initial implementation, I'll restrict elevated functions to debug mode (so I can get something up and running).


### Device

* Authorized functions on device
  + Token generation
* Uses
  + Jumping to bootloader
  + Reading keypresses directly
  + Features that can be dangerous if miss-used
* Token is granted per "Usage"
  + Selection of commands that comprise an action
* Should be valid, with an expiry due to:
  + Restarting HID-IO daemon
  + Restarting device
  + New connection to interface
  + Specific amount of time
  + At device request
  + At daemon request
* Device will only support a limited number of simultaneous tokens


### Daemon

* Authorized functions in daemon
  + Using requires generated token, physical device may not be available, or support token generation (bootloader mode)
* Uses
  + Unicode output
  + Simulated keyboard
  + Features that normally require system root priviledges
* Token is granted per "Usage"
  + Selection of commands that comprise an action
* Should be valid, with an expiry due to:
  + Restarting HID-IO daemon
  + New connection to interface
  + Specific amount of time
  + At daemon request


### Usage (is there a technical term already coined for this?)

A Usage is a collection of RPC functions that can define authentication terms.
* How long the authentication is valid
* Which RPC/functions may be called with the Usage
* Possibly the order in which functions are called
  + Calling out of order results in immediate revoking of the token
* Possibly revoking token if any RPC functions fail

