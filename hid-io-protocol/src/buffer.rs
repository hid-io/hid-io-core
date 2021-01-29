/* Copyright (C) 2020-2021 by Jacob Alexander
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in
 * all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
 * THE SOFTWARE.
 */

// ----- Crates -----

use super::*;
use heapless::spsc::Queue;
use heapless::Vec;

// ----- Enumerations -----

// ----- Structs -----

/// HID-IO byte buffer
/// This buffer is a queue of vecs with static allocation
/// Each vec is fixed sized as HID-IO interface
/// has a fixed transport payload (even if the actual size of the
/// message is less).
/// This buffer has no notion of packet size so it must store the
/// full transport payload.
/// In the minimal scenario a queue size of 1 is used.
///
/// Common HID-IO Vec capacities
/// - 7 bytes (USB 2.0 LS /w HID ID byte)
/// - 8 bytes (USB 2.0 LS)
/// - 63 bytes (USB 2.0 FS /w HID ID byte)
/// - 64 bytes (USB 2.0 FS)
/// - 1023 bytes (USB 2.0 HS /w HID ID byte)
/// - 1024 bytes (USB 2.0 HS)
///
/// The maximum queue size is 255
pub struct Buffer<Q: ArrayLength<Vec<u8, N>>, N: ArrayLength<u8>> {
    queue: Queue<Vec<u8, N>, Q, u8>,
}

// ----- Implementations -----

impl<Q, N> Default for Buffer<Q, N>
where
    Q: ArrayLength<Vec<u8, N>>,
    N: ArrayLength<u8>,
{
    fn default() -> Self {
        Buffer { queue: Queue::u8() }
    }
}

impl<Q: ArrayLength<Vec<u8, N>>, N: ArrayLength<u8>> Buffer<Q, N> {
    /// Constructor for Buffer
    ///
    /// # Remarks
    /// Initialize as blank
    /// This buffer has a limit of 65535 elements
    pub fn new() -> Buffer<Q, N> {
        Buffer {
            ..Default::default()
        }
    }

    /// Checks the first item array
    /// Returns None if there are no items in the queue
    /// Does not dequeue
    pub fn peek(&self) -> Option<&Vec<u8, N>> {
        self.queue.peek()
    }

    /// Dequeues and returns the first item array
    /// Returns None if there are no items in the queue
    pub fn dequeue(&mut self) -> Option<Vec<u8, N>> {
        self.queue.dequeue()
    }

    /// Enqueues
    /// Returns the array if there's not enough space
    pub fn enqueue(&mut self, data: Vec<u8, N>) -> Result<(), Vec<u8, N>> {
        self.queue.enqueue(data)
    }

    /// Clears the buffer
    /// Needed for some error conditions
    pub fn clear(&mut self) {
        while !self.queue.is_empty() {
            self.dequeue();
        }
    }

    /// Capacity of buffer
    pub fn capacity(&self) -> u8 {
        self.queue.capacity()
    }

    /// Number of elements stored in the buffer
    pub fn len(&self) -> u8 {
        self.queue.len()
    }

    /// Buffer empty
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// Buffer full
    pub fn is_full(&self) -> bool {
        self.len() == self.capacity()
    }
}
