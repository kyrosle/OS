//! App management syscalls
use crate::{batch::run_next_app, println};

/// task exits and submit an exit code.
pub fn sys_exit(xstate: i32) -> ! {
  println!("[kernel] Application exited with code {}", xstate);
  run_next_app()
}
