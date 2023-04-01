//! [kernel] Loading app_4
//! Try to access privileged CSR in U Mode
//! Kernel should kill this application!
//! [kernel] IllegalInstruction in application, kernel killed it

#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use riscv::register::sstatus::{self, SPP};

#[no_mangle]
fn main() -> i32 {
  println!("Try to access privileged CSR in U Mode");
  println!("Kernel should kill this application!");
  unsafe {
    sstatus::set_spp(SPP::User);
  }
  0
}
