#!/usr/bin/env python3

import asyncio
import logging
import sys

import hidio.client

logging.basicConfig(level=logging.DEBUG)
logger = logging.getLogger(__name__)


class MyHIDIOClient(hidio.client.HIDIOClient):
    async def on_connect(self, cap):
        logger.info("Connected!")
        print("Connected API Call", await cap.alive().a_wait())


    async def on_disconnect(self):
        logger.info("Disconnected!")


async def main():
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
            except asyncio.TimeoutError:
                logger.info("Alive timeout.")
                continue
        await asyncio.sleep(5)


try:
    loop = asyncio.get_event_loop()
    loop.run_until_complete(main())
except KeyboardInterrupt:
    logger.warning("Ctrl+C detected, exiting...")
    sys.exit(1)

