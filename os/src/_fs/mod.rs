use crate::mm::UserBuffer;

mod inode;
mod stdio;

pub use inode::{list_apps, open_file, OSInode, OpenFlags};
pub use stdio::{Stdin, Stdout};

/// File trait
pub trait File: Send + Sync {
  /// If readable
  fn readable(&self) -> bool;
  /// If writable
  fn writable(&self) -> bool;
  /// Read file to `UserBuffer`
  fn read(&self, buf: UserBuffer) -> usize;
  /// Write `UserBuffer` to file
  fn write(&self, buf: UserBuffer) -> usize;
}
