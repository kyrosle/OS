//! Memory management implementation
//!
//! SV39 page-based virtual-memory architecture for RV64 systems, and
//! everything about memory management, like frame allocator, page table,
//! map area and memory set, is implemented here.
//!
//! Every task or process has a memory_set to control its virtual memory.
mod address;
mod frame_allocator;
mod heap_allocator;
mod memory_set;
mod page_table;

pub use address::*;
pub use memory_set::*;
pub use page_table::*;

/// initialize heap allocator, frame allocator, and kernel space.
pub fn init() {
  heap_allocator::init_heap();
  println!("---- heap allocator testing start ----");
  heap_allocator::heap_test();
  println!("---- heap allocator testing end ----");

  frame_allocator::init_frame_allocator();
  // println!("---- frame allocator testing ----");
  // frame_allocator::frame_allocator_test();
  println!("1");
  KERNEL_SPACE.exclusive_access().activate();
}
