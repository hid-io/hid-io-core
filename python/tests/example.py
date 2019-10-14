#!/usr/bin/env python3
'''
Basic HID-IO Python Client Example
'''

# Copyright (C) 2019 Jacob Alexander
#
# Permission is hereby granted, free of charge, to any person obtaining a copy
# of this software and associated documentation files (the "Software"), to deal
# in the Software without restriction, including without limitation the rights
# to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
# copies of the Software, and to permit persons to whom the Software is
# furnished to do so, subject to the following conditions:
#
# The above copyright notice and this permission notice shall be included in
# all copies or substantial portions of the Software.
#
# THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
# IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
# FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
# AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
# LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
# OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
# THE SOFTWARE.

import argparse
import asyncio
import logging
import os
import sys

sys.path.append(os.path.join(os.path.dirname(__file__), '..'))
import hidio.client

logging.basicConfig(level=logging.DEBUG)
logger = logging.getLogger(__name__)


class MyHIDIOClient(hidio.client.HIDIOClient):
    async def on_connect(self, cap):
        logger.info("Connected!")
        print("Connected API Call", await cap.alive().a_wait())


    async def on_disconnect(self):
        logger.info("Disconnected!")


async def main(args):
    client = MyHIDIOClient('Python example.py')
    # Connect the client to the server using a background task
    # This will automatically reconnect
    tasks = [asyncio.gather(*[client.connect(auth=hidio.client.HIDIOClient.AUTH_BASIC)], return_exceptions=True)]
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
                # Check if this is just a single-shot test
                if args.single:
                    return
            except asyncio.TimeoutError:
                logger.info("Alive timeout.")
                continue
        await asyncio.sleep(5)


parser = argparse.ArgumentParser(description='Example HID-IO client library for Python')
parser.add_argument('--single', action='store_true')
args = parser.parse_args()
try:
    loop = asyncio.get_event_loop()
    loop.run_until_complete(main(args))
except KeyboardInterrupt:
    logger.warning("Ctrl+C detected, exiting...")
    sys.exit(1)
