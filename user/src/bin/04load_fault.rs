#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use core::ptr::{null_mut, read_volatile};

#[no_mangle]
fn main() -> i32 {
  println!("\nload_fault APP running...\n");
  println!("Into Test load_fault, we will insert an invalid load operation...");
  println!("Kernel should kill this application!");
  unsafe {
    let p: *mut u8 = null_mut();
    let _i = read_volatile(p);
  }
  0
}
