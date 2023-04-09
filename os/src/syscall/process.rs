//! Process management syscalls
use crate::{
  println,
  task::{exit_current_and_run_exit, suspend_current_and_run_next},
  timer::get_time_us,
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
  pub sec: usize,
  pub usec: usize,
}

/// task exits and submit an exit code.
pub fn sys_exit(xstate: i32) -> ! {
  println!("[kernel] Application exited with code {}", xstate);
  exit_current_and_run_exit();
  panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks.
pub fn sys_yield() -> isize {
  suspend_current_and_run_next();
  0
}

/// get time in milliseconds
pub fn sys_get_time() -> isize {
  get_time_us() as isize
}
