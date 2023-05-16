//! Types related to task(Thread) management
use core::cell::RefMut;

use super::{
  kstack_alloc, KernelStack, ProcessControlBlock,
  TaskContext, TaskUserRes,
};
use crate::{
  mm::PhysPageNum, sync::UPSafeCell, trap::TrapContext,
};
use alloc::sync::{Arc, Weak};

#[derive(Copy, Clone, PartialEq)]
pub enum TaskStatus {
  Ready,
  Running,
  Blocked,
}

/// Task control block structure(Thread)
pub struct TaskControlBlock {
  // immutable
  pub process: Weak<ProcessControlBlock>,
  pub kstack: KernelStack,

  // mutable
  inner: UPSafeCell<TaskControlBlockInner>,
}

impl TaskControlBlock {
  /// Create a new TCB (thread control block).
  /// - process: the process this thread belongs to.
  /// - ustack_base: the base address in same process space.
  /// - alloc_user_res: thread resources.
  pub fn new(
    process: Arc<ProcessControlBlock>,
    ustack_base: usize,
    alloc_user_res: bool,
  ) -> Self {
    let res = TaskUserRes::new(
      Arc::clone(&process),
      ustack_base,
      alloc_user_res,
    );
    let trap_cx_ppn = res.trap_cx_ppn();
    let kstack = kstack_alloc();
    let kstack_top = kstack.get_top();
    Self {
      process: Arc::downgrade(&process),
      kstack,
      inner: unsafe {
        UPSafeCell::new(TaskControlBlockInner {
          res: Some(res),
          trap_cx_ppn,
          task_cx: TaskContext::goto_trap_return(
            kstack_top,
          ),
          task_status: TaskStatus::Ready,
          exit_code: None,
        })
      },
    }
  }

  pub fn inner_exclusive_access(
    &self,
  ) -> RefMut<'_, TaskControlBlockInner> {
    self.inner.exclusive_access()
  }

  pub fn get_user_token(&self) -> usize {
    let process = self.process.upgrade().unwrap();
    let inner = process.inner_exclusive_access();
    inner.memory_set.token()
  }
}

pub struct TaskControlBlockInner {
  /// Thread resource set.
  pub res: Option<TaskUserRes>,
  /// Indicates the physical page number of the physical page frame,
  /// in which the Trap context in the application address space is placed.
  pub trap_cx_ppn: PhysPageNum,
  /// Save the task context of the suspended task in the task control block.
  pub task_cx: TaskContext,
  /// Maintain the execution state of the current process.
  pub task_status: TaskStatus,
  /// when the process system call `exit()` or execution meets error and terminated by kernel,
  /// `exit_code` will save in its TCB, and then waiting for recycle its resources by it's parent process
  /// by calling `waittid()`.
  pub exit_code: Option<i32>,
}

impl TaskControlBlockInner {
  pub fn get_trap_cx(&self) -> &'static mut TrapContext {
    self.trap_cx_ppn.get_mut()
  }

  #[allow(unused)]
  fn get_status(&self) -> TaskStatus {
    self.task_status
  }
}
