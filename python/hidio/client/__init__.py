'''
HID-IO Python Client Library
'''

# Copyright (C) 2019 by Jacob Alexander
#
# Permission is hereby granted, free of charge, to any person obtaining a copy
# of this software and associated documentation files (the "Software"), to deal
# in the Software without restriction, including without limitation the rights
# to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
# copies of the Software, and to permit persons to whom the Software is
# furnished to do so, subject to the following conditions:
#
# The above copyright notice and this permission notice shall be included in all
# copies or substantial portions of the Software.
#
# THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
# IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
# FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
# AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
# LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
# OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
# SOFTWARE.

## Imports
import asyncio
import logging
import os
import random
import sys
import socket
import ssl

## Capnp Imports
import capnp
import hidio.schema.hidio_capnp as hidio_capnp

# Logging
logger = logging.getLogger(__name__)


class HIDIOClient:
    '''
    HID-IO RPC interface class

    Handles socket reconnections for you.
    Generally returns an error if the socket is no longer available.
    '''

    AUTH_NONE = 'None'
    AUTH_BASIC = 'Basic'
    AUTH_ADMIN = 'Admin'

    def __init__(self, client_name):
        '''
        Initializes socket connection and capnproto schemas

        @param client_name: Name of the client, used for logging/info
        '''
        self.retry_task = None
        self.retry_connection = True
        self.addr = None
        self.port = None
        self.ctx = None
        self.reader = None
        self.writer = None
        self.client = None
        self.cap = None
        self.cap_auth = None
        self.overalltasks = []
        self.auth = self.AUTH_NONE
        self.loop = None
        self.version_info = None
        self.client_name = client_name


    def __del__(self):
        '''
        Forceably cancel all async tasks when deleting the object
        '''
        # Make sure we have a reference to the running loop
        if not self.loop:
            self.loop = asyncio.get_event_loop()
        asyncio.ensure_future(self.disconnect(), loop=self.loop)


    async def socketreader(self):
        '''
        Reads from asyncio socket and writes to pycapnp client interface
        '''
        while self.retry_task:
            try:
                # Must be a wait_for in order to give watch_connection a slot
                # to try again
                data = await asyncio.wait_for(
                    self.reader.read(4096),
                    timeout=5.0
                )
            except asyncio.TimeoutError:
                logger.debug("socketreader timeout.")
                continue
            self.client.write(data)
        return True


    async def socketwriter(self):
        '''
        Reads from pycapnp client interface and writes to asyncio socket
        '''
        while self.retry_task:
            try:
                # Must be a wait_for in order to give watch_connection a slot
                # to try again
                data = await asyncio.wait_for(
                    self.client.read(4096),
                    timeout=5.0
                )
                self.writer.write(data.tobytes())
            except asyncio.TimeoutError:
                logger.debug("socketwriter timeout.")
                continue
        return True


    async def socketwatcher(self):
        '''
        Periodically attempts to make an API call with a timeout to validate
        the server is still alive
        '''
        while self.retry_task:
            try:
                await asyncio.wait_for(
                    self.cap.alive().a_wait(),
                    timeout=1.0
                )
                logger.debug("Server connection ok.")
                await asyncio.sleep(2)
            except asyncio.TimeoutError:
                logging.debug("Server connection failed, disconnecting.")
                # End other tasks
                await self.disconnect(retry_connection=True)
                return False
        return True


    async def socketconnection(self):
        '''
        Main socket connection function
        May be called repeatedly when trying to open a connection
        '''
        # Make sure we retry tasks on reconnection
        self.retry_task = True

        # Setup SSL context
        self.ctx = ssl.SSLContext()

        # Handle both IPv4 and IPv6 cases
        try:
            logging.debug("Try IPv4 (may autodetect IPv6)")
            self.reader, self.writer = await asyncio.open_connection(
                self.addr, self.port,
                ssl=self.ctx,
            )
        except OSError:
            logging.debug("Try IPv6")
            try:
                self.reader, self.writer = await asyncio.open_connection(
                    self.addr, self.port,
                    ssl=self.ctx,
                    family=socket.AF_INET6
                )
            except OSError:
                logger.debug("Retrying port connection {}:{} auth level {}".format(self.addr, self.port, self.auth))
                return False

        self.overalltasks = []

        # Assemble reader and writer tasks, run in the background
        logging.debug("Backgrounding socket reader and writer functions")
        coroutines = [self.socketreader(), self.socketwriter()]
        self.overalltasks.append(asyncio.gather(*coroutines, return_exceptions=True))

        # Start TwoPartyClient using TwoWayPipe (takes no arguments in this mode)
        logging.debug("Starting TwoPartyClient")
        self.client = capnp.TwoPartyClient()
        logging.debug("Starting Bootstrap")
        self.cap = self.client.bootstrap().cast_as(hidio_capnp.HIDIOServer)

        # Start watcher to restart socket connection if it is lost
        logging.debug("Backgrounding socketwatcher")
        watcher = [self.socketwatcher()]
        self.overalltasks.append(asyncio.gather(*watcher, return_exceptions=True))

        # Lookup version information
        self.version_info = (await self.cap.version().a_wait()).version
        logger.info(self.version_info)

        # Lookup uid
        self.uid_info = (await self.cap.id().a_wait()).id
        logger.info("uid: %s", self.uid_info)

        # AUTH_NONE doesn't need to go any further
        if self.auth:
            # Lookup key information
            self.key_info = (await self.cap.key().a_wait()).key
            logger.info(self.key_info)

            # Lookup key for auth level
            key_lookup = {
                self.AUTH_BASIC: self.key_info.basicKeyPath,
                self.AUTH_ADMIN: self.key_info.authKeyPath,
            }
            key_location = key_lookup[self.auth]

            # Fail connection if authentication key cannot be read
            # This usually means that the client doesn't have permission
            self.key = None
            try:
                with open(key_location, 'r') as myfile:
                    self.key = myfile.read()
            except OSError as err:
                logger.error("Could not read '%s'. This usually means insufficient permissions.", key_location)
                logger.error(err)
                await self.disconnect()
                return False
            logger.info("Key: %s", self.key)

            # Connect to specified auth level
            key_usage = {
                self.AUTH_BASIC: self.cap.basic_request(),
                self.AUTH_ADMIN: self.cap.auth_request(),
            }

            request = key_usage[self.auth]
            request.key = self.key
            request.info.type = 'hidioApi'
            request.info.name = self.client_name
            request.info.serial = "{}".format(random.getrandbits(64)) # Just use a random number
            request.info.id = self.uid_info

            # Validate auth key
            try:
                self.cap_auth = (await request.send().a_wait()).port
            except:
                logger.error("Invalid auth key!")
                await self.disconnect()
                return False
            logger.debug("Authenticated with %s", self.auth)

        # Callback
        await self.on_connect(self.cap)

        # Spin here until connection is broken
        while self.retry_task:
            await asyncio.sleep(1)


    async def connect(self, auth=AUTH_NONE, addr='localhost', port='7185'):
        '''
        Attempts to reconnect to the secured port
        Will gather keys for interfaces
        '''
        self.addr = addr
        self.port = port
        self.auth = auth
        self.loop = asyncio.get_event_loop()
        logger.info("Connecting to {}:{} with auth level {}".format(self.addr, self.port, self.auth))

        # Enable task and connection retries
        self.retry_task = True
        self.retry_connection = True

        # Continue to reconnect until specified to stop
        while self.retry_connection:
            try:
                await self.socketconnection()
            except Exception as err:
                logger.error("Unhandled Exception")
                logger.error(err)
            await asyncio.sleep(1)

        # Remove reference to loop once we finish
        self.loop = None


    async def disconnect(self, retry_connection=False):
        '''
        Forceably disconnects the server

        @param retry_connection: If set to True, will attempt to reconnect to server
        '''
        logger.info("Disconnecting from {}:{} (auth level {})".format(self.addr, self.port, self.auth))

        # Callback
        await self.on_disconnect()

        # Gently end tasks (should not delay more than 5 seconds)
        self.retry_task = False
        logger.debug("Tasks open: %s", len(self.overalltasks))
        for index, task in enumerate(self.overalltasks):
            logger.debug("Ending task: %s", index)
            await task

        # Cleanup state
        self.reader = None
        self.writer = None
        self.ctx = None
        self.client = None
        self.cap = None
        self.cap_auth = None
        self.version_info = None

        # Stop retrying connection if specified
        if retry_connection:
            logger.debug("Retrying connection.")
            return
        logger.debug("Stopping client.")
        self.retry_connection = False



    async def on_connect(self, cap):
        '''
        This callback is meant to be overridden
        It is called on server connection events.
        This may occur if the server restarts, or due to some network issue.
        '''


    async def on_disconnect(self):
        '''
        This callback is meant to be overridden
        It is called on server disconnection events.
        This may occur if the server dies, or due to some network issue.
        '''


    def capability_hidioserver(self):
        '''
        Returns a reference to the capability
        This will be refreshed on each on_connect callback event.
        '''
        return self.cap


    def capability_authenticated(self):
        '''
        Returns a reference to the authenticated capability
        This will return None if not authenticated.
        '''
        return self.cap_auth


    def retry_connection_status(self):
        '''
        Returns whether or not connection retry is enabled
        Certain events will turn this off (Ctrl+C, bad auth level)
        Use this to stop the event loop.
        '''
        return self.retry_connection


    def version(self):
        '''
        If connected successfully, will return version information

        For example:
        ( version = "0.1.0-beta (git v0.1.0-beta-12-ge5d51a6)",
          buildtime = "Wed, 09 Oct 2019 06:46:18 +0000",
          serverarch = "x86_64-apple-darwin",
          compilerversion = "rustc 1.38.0-nightly (9703ef666 2019-08-10)" )
        '''
        return self.version_info

