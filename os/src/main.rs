//! The main module and entrypoint.
//!
//! Various facilities of the kernels are implemented as submodules.
//! The most important ones are:
//!
//! - [`trap`]: Handles all cases of switching from user-space to the kernel.
//! - [`task`]: Task management.
//! - [`syscall`]: System call handling and implementation.
//! - [`mm`]: Address map using SV39
//! - [`sync`]: Wrap a static data structure inside it so that we are able to
//!             access it without any `unsafe`.
//! - [`fs`]: Separate user from file system with some structure.
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

#[macro_use]
mod console;
mod config;
mod fs;
mod lang_items;
mod mm;
mod sbi;
mod sync;
mod syscall;
mod task;
mod timer;
mod trap;

mod drivers;
mod qemu;

// Embed this assembly code.
global_asm!(include_str!("entry.asm"));
// this asm source file is created by build.rs
// global_asm!(include_str!("link_app.S"));

#[no_mangle]
/// The rust entry-point of os
pub fn rust_main() -> ! {
  clear_bss();
  println!("[kernel] Kernel started.");
  println!("[kernel] memory init.");
  mm::init();
  println!("[kernel] memory test.");
  mm::remap_test();
  println!("[kernel] trap init.");
  trap::init();
  println!("[kernel] timer interrupt enable.");
  trap::enable_timer_interrupt();
  println!("[kernel] timer interrupt set.");
  timer::set_next_trigger();
  println!("[kernel] list application.");
  fs::list_apps();
  println!("[kernel] add init process.");
  task::add_initproc();
  println!("[kernel] run tasks.");
  task::run_tasks();
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
