#!/usr/bin/env python3
'''
Basic HID-IO Python Client Example
'''

# Copyright (C) 2019-2020 Jacob Alexander
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
import time

sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..'))
import hidiocore.client

logging.basicConfig(level=logging.DEBUG)
logger = logging.getLogger(__name__)


class MyHidIoClient(hidiocore.client.HidIoClient):
    async def on_connect(self, cap, cap_auth):
        logger.info("Connected!")
        print("Connected API Call", await cap.alive().a_wait())


    async def on_disconnect(self):
        logger.info("Disconnected!")


async def main(args):
    client = MyHidIoClient('Python info gathering example')
    # Connect the client to the server using a background task
    # This will automatically reconnect
    tasks = [asyncio.gather(*[client.connect(auth=hidiocore.client.HidIoClient.AUTH_BASIC)], return_exceptions=True)]
    while client.retry_connection_status():
        if client.capability_hidioserver():
            try:
                # Get list of nodes
                nodes = (await asyncio.wait_for(
                    client.nodes(),
                    timeout=2.0
                )).nodes

                # Match on HidioDaemon node
                nodes = [n for n in nodes if n.type == 'hidioDaemon']
                assert len(nodes) == 1, "There can be only one! ...hidioDaemon"

                # Print daemon information
                print(await nodes[0].node.daemon.info().a_wait())

                return
            except asyncio.TimeoutError:
                logger.info("Timeout, trying again.")
                continue
        await asyncio.sleep(1)


parser = argparse.ArgumentParser(description='Info gathering example for HID-IO')
args = parser.parse_args()
try:
    loop = asyncio.get_event_loop()
    loop.run_until_complete(main(args))
except KeyboardInterrupt:
    logger.warning("Ctrl+C detected, exiting...")
    sys.exit(1)
