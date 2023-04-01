//! The main module and entrypoint.
//! 
//! Various facilities of the kernels are implemented as submodules. 
//! The most important ones are:
//! 
//! - [`trap`]: Handles all cases of switching from user-space to the kernel.
//! - [`syscall`]: System call handling and implementation.
//! 
//! The operating system also starts in this module. Kernel code starts
//! executing from `entry.asm`, after which [`rust_main()`] is called to
//! initialize various pieces of functionality.
//! 
//! We then call [`batch::run_next_app()`] and for the first time go to user-space.

#![no_std]
#![no_main]
#![feature(panic_info_message)]

use core::arch::global_asm;

mod batch;
#[macro_use]
mod console;
mod lang_items;
mod sbi;
mod sync;
mod syscall;
mod trap;
mod stack_trace;

// Embed this assembly code.
global_asm!(include_str!("entry.asm"));
// this asm source file is created by build.rs
global_asm!(include_str!("link_app.S"));

/// The rust entry-point of os
#[no_mangle]
pub fn rust_main() -> ! {
  clear_bss();
  println!("[kernel] Hello,world!");
  trap::init();
  batch::init();
  batch::run_next_app();
}

/// Clear BSS segment
fn clear_bss() {
  extern "C" {
    fn sbss();
    fn ebss();
  }
  (sbss as usize..ebss as usize)
    .for_each(|a| unsafe { (a as *mut u8).write_volatile(0) })
}
