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

