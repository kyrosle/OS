//! [kernel] Loading app_0
//! Hello, world!
//! [kernel] Application exited with code 0

#![no_std]
#![no_main]
#[macro_use]
extern crate user_lib;

#[no_mangle]
fn main() -> i32 {
  println!("Hello, world!");
  0
}
