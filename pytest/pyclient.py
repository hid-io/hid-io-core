#!/usr/bin/env python3
# Jacob Alexander 2017

import capnp
import sys

sys.path.append("../schema")

import hidio_capnp

print("Client!")

# Get hidio capability after bootstrapping
client = capnp.TwoPartyClient('127.0.0.1:7185') # 0x1c11
cap_bootstrap = client.bootstrap()
cap = cap_bootstrap.cast_as(hidio_capnp.HIDIOServer)

# Retrieve hidio (i.e. authenticate client with server)
remote = cap.basic()
response = remote.wait()
hidio = response.port

# List nodes
# And register signals
# TODO Pipeline registrations as an example
nodes = hidio.nodes().wait()
print( nodes )

for node in nodes.nodes:
    print( node.type, node.node.register().wait() )


# Call signal
print( hidio )
remote = hidio.signal( 27 )
response = remote.wait()
print( response )

remote = hidio.signal( 40 )
response = remote.wait()
print( response )


#remote = cap.foo(i=5)
#response = remote.wait()

#assert response.x == '125'
#print(response, response.x)
#remote = cap.signal()
#remote = cap.test()
#print( remote )


