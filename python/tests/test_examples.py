'''
Basic test cases for HID-IO Client Python Library
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

import os
import socket
import subprocess
import sys
import time

python_dir = os.path.join(os.path.dirname(__file__))

def run_subprocesses(client, args):
    server = subprocess.Popen(['cargo', 'run'], cwd=os.path.join(python_dir, '..', '..'))

    # Wait for server to start
    addr, port = ('localhost', 7185)
    retries = 30
    while True:
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        result = sock.connect_ex((addr, port))
        if result == 0:
            break
        sock = socket.socket(socket.AF_INET6, socket.SOCK_STREAM)
        result = sock.connect_ex((addr, port))
        if result == 0:
            break
        # Give the server some small amount of time to start listening
        time.sleep(1)
        retries -= 1
        if retries == 0:
            assert False, "Timed out waiting for server to start"
    client_args = [sys.executable, os.path.join(python_dir, client)]
    client_args.extend(args)
    client = subprocess.Popen(client_args)

    ret = client.wait()
    server.kill()
    assert ret == 0, "Client did not return 0"


def test_example():
    client = 'example.py'
    args = ['--single']
    run_subprocesses(client, args)
