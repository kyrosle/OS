use alloc::sync::{Arc, Weak};

use super::File;
use crate::{
  sync::UPSafeCell, task::suspend_current_and_run_next,
};

/// Maximum size of pipe ring buffer.
const RING_BUFFER_SIZE: usize = 32;

/// The abstract one end of the pipe (read or write).
pub struct Pipe {
  readable: bool,
  writable: bool,
  buffer: Arc<UPSafeCell<PipeRingBuffer>>,
}

impl Pipe {
  /// Set the pipe only readable.
  pub fn read_end_with_buffer(
    buffer: Arc<UPSafeCell<PipeRingBuffer>>,
  ) -> Self {
    Self {
      readable: true,
      writable: false,
      buffer,
    }
  }

  /// Set the pipe only writable.
  pub fn write_end_with_buffer(
    buffer: Arc<UPSafeCell<PipeRingBuffer>>,
  ) -> Self {
    Self {
      readable: false,
      writable: true,
      buffer,
    }
  }
}

impl File for Pipe {
  fn readable(&self) -> bool {
    self.readable
  }

  fn writable(&self) -> bool {
    self.writable
  }

  fn read(&self, buf: crate::mm::UserBuffer) -> usize {
    assert!(self.readable());
    let want_to_read = buf.len();
    // convert `UserBuffer` into a iterator, conveniently fetch a raw byte pointer in order.
    let mut buf_iter = buf.into_iter();
    // maintain how many bytes are actually read from the pipe into the application's buffer.
    let mut already_read = 0usize;
    // This may exceed the size of the circular queue,
    // or since no process has yet written enough characters from the write end of the pipeline,
    // we need to put the entire reading process in a loop,
    // when there are not enough characters in the circular queue, we should temporarily switch tasks and
    // wait for the characters in the circular queue to be replenished before continuing to read.
    loop {
      let mut ring_buffer = self.buffer.exclusive_access();
      let loop_read = ring_buffer.available_read();
      // if the pipe is empty.
      if loop_read == 0 {
        // check the all write_end whether closed.
        if ring_buffer.all_write_ends_closed() {
          return already_read;
        }
        // manually release the lock, cos of switching task `__switch` run out of function bounded.
        drop(ring_buffer);
        suspend_current_and_run_next();
        continue;
      }
      // iterates over each byte pointer in the buffer, and read by `PipeRigBuffer::read_byte` from pipe.
      for _ in 0..loop_read {
        if let Some(byte_ref) = buf_iter.next() {
          unsafe {
            *byte_ref = ring_buffer.read_byte();
          }
          already_read += 1;
          if already_read == want_to_read {
            return want_to_read;
          }
        } else {
          return already_read;
        }
      }
    }
  }

  fn write(&self, buf: crate::mm::UserBuffer) -> usize {
    assert!(self.writable());
    let want_to_write = buf.len();
    let mut buf_iter = buf.into_iter();
    let mut already_write = 0usize;
    loop {
      let mut ring_buffer = self.buffer.exclusive_access();
      let loop_write = ring_buffer.available_write();
      if loop_write == 0 {
        drop(ring_buffer);
        suspend_current_and_run_next();
        continue;
      }
      // write at most loop_write bytes
      for _ in 0..loop_write {
        if let Some(byte_ref) = buf_iter.next() {
          ring_buffer.write_byte(unsafe { *byte_ref });
          already_write += 1;
          if already_write == want_to_write {
            return want_to_write;
          }
        } else {
          return already_write;
        }
      }
    }
  }
}

#[derive(Copy, Clone, PartialEq)]
/// Recording the current Pipe status.
enum RingBufferStatus {
  /// Buffer is full, cannot be written.
  Full,
  /// Buffer is empty, cannot be read.
  Empty,
  /// Other status.
  Normal,
}

pub struct PipeRingBuffer {
  /// data storage
  arr: [u8; RING_BUFFER_SIZE],
  /// ring buffer head index.
  head: usize,
  /// ring buffer tail index.
  tail: usize,
  status: RingBufferStatus,
  /// weak reference to write-end, checking whether the write-end have been closed.
  write_end: Option<Weak<Pipe>>,
}

impl PipeRingBuffer {
  /// Create a new PipeRingBuffer in Empty status, and doesn't have `write_end`.
  pub fn new() -> Self {
    Self {
      arr: [0; RING_BUFFER_SIZE],
      head: 0,
      tail: 0,
      status: RingBufferStatus::Empty,
      write_end: None,
    }
  }

  /// Acquire the weak reference of write_end and storing it.
  pub fn set_write_end(&mut self, write_end: &Arc<Pipe>) {
    self.write_end = Some(Arc::downgrade(write_end));
  }

  /// Write a byte to the ring buffer.
  pub fn write_byte(&mut self, byte: u8) {
    self.status = RingBufferStatus::Normal;
    self.arr[self.tail] = byte;
    self.tail = (self.tail + 1) % RING_BUFFER_SIZE;
    if self.tail == self.head {
      self.status = RingBufferStatus::Full;
    }
  }

  /// Read a bytes(u8) from the pipe, it should not empty before calling this method.
  ///
  /// if head index is equal tail index, the status may EMPTY or FULL, so we should
  /// update the status while calling `read_byte`.
  pub fn read_byte(&mut self) -> u8 {
    self.status = RingBufferStatus::Normal;
    let c = self.arr[self.head];
    self.head = (self.head + 1) % RING_BUFFER_SIZE;
    if self.head == self.tail {
      self.status = RingBufferStatus::Empty;
    }
    c
  }

  /// Calculate the count of bytes can be read.
  pub fn available_read(&self) -> usize {
    if self.status == RingBufferStatus::Empty {
      0
    } else if self.tail > self.head {
      self.tail - self.head
    } else {
      self.tail + RING_BUFFER_SIZE - self.head
    }
  }

  pub fn available_write(&self) -> usize {
    if self.status == RingBufferStatus::Full {
      0
    } else {
      RING_BUFFER_SIZE - self.available_read()
    }
  }

  /// Check whether all write_end are closed.
  ///
  /// This is trying to update the field: `write_end` (weak reference) to strong reference.
  /// If the upgrade is failed, the `write_end` strong reference count is 0, all write_end have been closed,
  /// so that the data in the pipeline will no longer be replenished, after the only remaining data in the pipeline
  /// has been read, the pipeline can be destroyed.
  pub fn all_write_ends_closed(&self) -> bool {
    self.write_end.as_ref().unwrap().upgrade().is_none()
  }
}

/// Return (read_end, write_end)
pub fn make_pipe() -> (Arc<Pipe>, Arc<Pipe>) {
  let buffer = Arc::new(unsafe {
    UPSafeCell::new(PipeRingBuffer::new())
  });
  let read_end =
    Arc::new(Pipe::read_end_with_buffer(buffer.clone()));
  let write_end =
    Arc::new(Pipe::write_end_with_buffer(buffer.clone()));
  buffer.exclusive_access().set_write_end(&write_end);
  (read_end, write_end)
}
