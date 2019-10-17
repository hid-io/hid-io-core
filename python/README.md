# hidio core Client Python Library
HID-IO Core Client Side Library for Python

[![Linux Status](https://github.com/hid-io/hid-io/workflows/Rust%20Linux/badge.svg)](https://github.com/hid-io/hid-io/actions)
[![macOS Status](https://github.com/hid-io/hid-io/workflows/Rust%20macOS/badge.svg)](https://github.com/hid-io/hid-io/actions)
[![Windows Status](https://github.com/hid-io/hid-io/workflows/Rust%20Windows/badge.svg)](https://github.com/hid-io/hid-io/actions)

[![Visit our IRC channel](https://kiwiirc.com/buttons/irc.freenode.net/hid-io.png)](https://kiwiirc.com/client/irc.freenode.net/#hid-io)

## Getting

```bash
pip install hidiocore
```


## Overview

This is a convenience Python library for the HID-IO daemon which handles automatic reconnection if the server goes down for any reason.
The library also handles the HID-IO authentication procedure (key negotiation and TLS wrapping).


## Usage

```python
import asyncio
import sys

import hidiocore.client

# Optional callbacks
class MyHIDIOClient(hidiocore.client.HIDIOClient):
    async def on_connect(self, cap):
        print("Connected!")
        print("Connected API Call", await cap.alive().a_wait())


    async def on_disconnect(self):
        print("Disconnected!")


async def main():
    client = MyHIDIOClient('Python example.py')
    # Connect the client to the server using a background task
    # This will automatically reconnect
    tasks = [asyncio.gather(*[client.connect(auth=hidiocore.client.HIDIOClient.AUTH_BASIC)], return_exceptions=True)]
    while client.retry_connection_status():
        if client.capability_hidioserver():
            try:
                print("API Call", await asyncio.wait_for(
                    client.capability_hidioserver().alive().a_wait(),
                    timeout=2.0
                ))
                print("API Call", await asyncio.wait_for(
                    client.capability_authenticated().nodes().a_wait(),
                    timeout=2.0
                ))
            except asyncio.TimeoutError:
                print("Alive timeout.")
                continue
        await asyncio.sleep(5)


try:
    loop = asyncio.get_event_loop()
    loop.run_until_complete(main())
except KeyboardInterrupt:
    print("Ctrl+C detected, exiting...")
    sys.exit(1)
```
