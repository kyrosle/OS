//! The main module and entrypoint.
//!
//! Various facilities of the kernels are implemented as submodules.
//! The most important ones are:
//!
//! - [`trap`]: Handles all cases of switching from user-space to the kernel.
//! - [`task`]: Task management.
//! - [`syscall`]: System call handling and implementation.
//!
//! The operating system also starts in this module. Kernel code starts
//! executing from `entry.asm`, after which [`rust_main()`] is called to
//! initialize various pieces of functionality.
//!
//! We then call [`task::run_first_task()`] and for the first time go to user-space.

#![no_std]
#![no_main]
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]

use core::arch::global_asm;
extern crate alloc;
#[macro_use]
extern crate bitflags;

mod loader;
#[macro_use]
mod console;
mod config;
mod lang_items;
mod mm;
mod sbi;
mod stack_trace;
mod sync;
mod syscall;
mod task;
mod timer;
mod trap;

// Embed this assembly code.
global_asm!(include_str!("entry.asm"));
// this asm source file is created by build.rs
global_asm!(include_str!("link_app.S"));

#[no_mangle]
/// The rust entry-point of os
pub fn rust_main() -> ! {
  clear_bss();
  println!("[kernel] ---- Kernel program startup ----");

  println!(
    "[kernel] ---- Initialize heap,frame allocator; kernel space;  ----"
  );
  mm::init();

  println!("[kernel] --- memory manager testing ---");
  mm::remap_test();

  println!("[kernel] --- Initialize Trap entry ---");
  trap::init();

  // setting `sie.stie` interruption won't be masked.
  trap::enable_timer_interrupt();

  // setting a 10 ms time counter.
  println!("[kernel] --- set up timer ---");
  timer::set_next_trigger();

  println!("[kernel] --- run the first task ---");
  task::run_first_task();

  panic!("Unreachable in rust_main!");
}

/// Clear BSS segment
fn clear_bss() {
  extern "C" {
    fn sbss();
    fn ebss();
  }
  unsafe {
    core::slice::from_raw_parts_mut(
      sbss as usize as *mut u8,
      ebss as usize - sbss as usize,
    )
    .fill(0);
  }
}
