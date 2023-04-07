//! Implementation of [`TaskContext`]

use crate::trap::trap_return;

#[repr(C)]
/// Task Context structure containing some registers.
pub struct TaskContext {
  /// return address ( e.g. __restore ) of __switch ASM function
  ra: usize,
  /// kernel stack pointer of app
  sp: usize,
  /// callee saved registers:  s0 ~ s11
  s: [usize; 12],
}

impl TaskContext {
  /// init task context
  pub fn zero_init() -> Self {
    TaskContext {
      ra: 0,
      sp: 0,
      s: [0; 12],
    }
  }

  /// set Task context{__restore ASM function: trap_return, sp: kstack_ptr, s: s0 ~ s12}
  pub fn goto_trap_return(kstack_ptr: usize) -> Self {
    TaskContext {
      ra: trap_return as usize,
      sp: kstack_ptr,
      s: [0; 12],
    }
  }
}
