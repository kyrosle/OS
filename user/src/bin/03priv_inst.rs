//! [kernel] Loading app_3
//! Try to execute privileged instruction in U Mode
//! Kernel should kill this application!
//! [kernel] IllegalInstruction in application, kernel killed it

#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use core::arch::asm;

#[no_mangle]
fn main() -> i32 {
  println!("Try to execute privileged instruction in U Mode");
  println!("Kernel should kill this application!");
  unsafe {
    asm!("sret");
  }
  0
}
