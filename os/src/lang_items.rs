//! The panic handler

use crate::stack_trace::print_stack_trace;
use crate::{println, sbi::shutdown};
use core::panic::PanicInfo;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
  if let Some(location) = info.location() {
    println!(
      "Panicked at {}:{} {}",
      location.file(),
      location.line(),
      info.message().unwrap()
    );
  } else {
    println!("Panicked: {}", info.message().unwrap())
  }
  unsafe {
    print_stack_trace();
  }
  shutdown()
}
