#!/usr/bin/env python3

import capnp
import sys

sys.path.append("../schema")

import hidio_capnp
import common_capnp

import hidiowatcher_capnp
import hostmacro_capnp
import usbkeyboard_capnp

print("Server!")

class HIDIOServerImpl( hidio_capnp.HIDIOServer.Server ):
    def __init__( self ):
        print("HIDIOServer Init")

    def basic( self, _context, **kwargs ):
        # Allocate HIDIO object for basic authentication
        return HIDIOImpl()

class HIDIOImpl( hidio_capnp.HIDIO.Server ):
    def __init__( self ):
        print("HIDIO Init")

        # TODO allocate nodes based on authentication level
        # Each nodes has it's own allocated destination
        # TODO Destinations should be queried, not generated
        #for node in [ USBKeyboardImpl(), HostMacroImpl() ]:
        usbkbd = common_capnp.Destination.new_message()
        usbkbd.type = 'usbKeyboard'
        usbkbd.name = "Test Keyboard"
        usbkbd.serial = "1467"
        usbkbd.id = 78500
        usbkbd.node = USBKeyboardImpl( self )

        hostmacro = common_capnp.Destination.new_message()
        hostmacro.type = 'hidioScript'
        hostmacro.name = "Test Script"
        hostmacro.serial = "A&d342"
        hostmacro.id = 99382569
        hostmacro.node = HostMacroImpl( self )

        self.authenticated_nodes = [ usbkbd, hostmacro ]

    def signal( self, time, _context, **kwargs ):
        #signal @0 (time :UInt64) -> (time :UInt64, signal :List(Signal));

        print("TODO signal", time)
        # TODO
        time = 10
        signal = [ self.new_signal() ]

        return (time, signal)

    def new_signal( self ):
        # TODO
        signal = hidio_capnp.HIDIO.Signal.new_message()

        # Time
        signal.time = 15

        # Source
        signal.source.type = 'usbKeyboard'
        signal.source.name = "Test usbkeyboard signal source"
        signal.source.serial = "SERIAL NUMBER!"
        signal.source.id = 1234567

        # Type
        signal.type.usbKeyboard.keyEvent.event = 'press'
        signal.type.usbKeyboard.keyEvent.id = 32

        return signal

    def nodes( self, **kwargs ):
        print("nodes")
        return self.authenticated_nodes

class USBKeyboardImpl( usbkeyboard_capnp.USBKeyboard.Server ):
    def __init__( self, hidio ):
        self.hidio = hidio

        print("USBKeyboard Init")
        self.registered = False

    def register( self, **kwargs ):
        print("Registered USBKeyboard")
        self.registered = True
        return True

    def isRegistered( self, **kwargs ):
        print("Is registered USBKeyboard")
        return self.registered

class HostMacroImpl( hostmacro_capnp.HostMacro.Server ):
    def __init__( self, hidio ):
        self.hidio = hidio

        print("HostMacro Init")
        self.registered = False

    def register( self, **kwargs ):
        print("Registered HostMacro")
        self.registered = True
        return True

    def isRegistered( self, **kwargs ):
        print("Is registered HostMacro")
        return self.registered

class HIDIOWatcherImpl( hidiowatcher_capnp.HIDIOWatcher.Server ):
    def __init__( self ):
        print("HIDIOWatcher Init")

server = capnp.TwoPartyServer( '127.0.0.1:7185', bootstrap=HIDIOServerImpl() ) # 0x1c11

server.run_forever()

