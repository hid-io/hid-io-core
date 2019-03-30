#!/usr/local/bin python

from cffi import FFI
ffi = FFI()
ffi.cdef("""
    int hello_world(int);
""")

C = ffi.dlopen("./target/debug/libhid_io.so")

print(C.hello_world(9))
