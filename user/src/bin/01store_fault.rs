//! [kernel] Loading app_1
//! Into Test store_fault, we will insert an invalid store operation...
//! Kernel should kill this application!
//! [kernel] PageFault in application, kernel killed it
#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

#[no_mangle]
fn main() -> i32 {
  println!(
    "Into Test store_fault, we will insert an invalid store operation..."
  );
  println!("Kernel should kill this application!");
  unsafe {
    core::ptr::null_mut::<u8>().write_volatile(0);
  }
  0
}
