use alloc::sync::Arc;

use crate::{sync::UPSafeCell, trap::TrapContext};

use super::{
  manager::fetch_task, switch::__switch,
  ProcessControlBlock, TaskContext, TaskControlBlock,
  TaskStatus,
};
use lazy_static::lazy_static;

lazy_static! {
  pub static ref PROCESSOR: UPSafeCell<Processor> =
    unsafe { UPSafeCell::new(Processor::new()) };
}

/// Processor management structure
pub struct Processor {
  /// The task currently executing on the current processor
  current: Option<Arc<TaskControlBlock>>,
  /// The basic control flow of each core, helping to select and switch process
  idle_task_cx: TaskContext,
}

impl Processor {
  /// Create a empty processor
  pub fn new() -> Self {
    Self {
      current: None,
      idle_task_cx: TaskContext::zero_init(),
    }
  }

  /// Get mutable reference to `idle_task_cx`
  fn get_idle_task_cx_ptr(&mut self) -> *mut TaskContext {
    &mut self.idle_task_cx as *mut _
  }

  /// Get current task in moving semanteme
  pub fn take_current(
    &mut self,
  ) -> Option<Arc<TaskControlBlock>> {
    self.current.take()
  }

  /// Get current task in cloning semanteme
  pub fn current(&self) -> Option<Arc<TaskControlBlock>> {
    self.current.as_ref().map(Arc::clone)
  }
}

/// The main part of process execution and scheduling
/// Loop `fetch_task` to get the process that needs to run, and switch the process through `__switch`
pub fn run_tasks() {
  loop {
    let mut processor = PROCESSOR.exclusive_access();
    if let Some(task) = fetch_task() {
      let idle_task_cx_ptr =
        processor.get_idle_task_cx_ptr();
      // access coming task TCB exclusively
      let mut task_inner = task.inner_exclusive_access();
      let next_task_cx_ptr =
        &task_inner.task_cx as *const TaskContext;
      task_inner.task_status = TaskStatus::Running;
      drop(task_inner);
      // release coming task TCB manually
      processor.current = Some(task);
      // release processor manually
      drop(processor);
      unsafe {
        __switch(idle_task_cx_ptr, next_task_cx_ptr);
      }
    } else {
      println!("no tasks available in run_tasks");
    }
  }
}

/// Take the current task, leaving a None in its place.
pub fn take_current_task() -> Option<Arc<TaskControlBlock>>
{
  PROCESSOR.exclusive_access().take_current()
}

/// Get running task.
pub fn current_task() -> Option<Arc<TaskControlBlock>> {
  PROCESSOR.exclusive_access().current()
}

/// Get current process from current running task.
pub fn current_process() -> Arc<ProcessControlBlock> {
  current_task().unwrap().process.upgrade().unwrap()
}

/// Get token of the address space of current task
pub fn current_user_token() -> usize {
  let task = current_task().unwrap();
  task.get_user_token()
}

/// Get the mutable reference to trap context of current task
pub fn current_trap_cx() -> &'static mut TrapContext {
  current_task()
    .unwrap()
    .inner_exclusive_access()
    .get_trap_cx()
}

/// Get the current TrapContext by virtual address.
pub fn current_trap_cx_user_va() -> usize {
  current_task()
    .unwrap()
    .inner_exclusive_access()
    .res
    .as_ref()
    .unwrap()
    .trap_cx_user_va()
}

/// Acquire the current kernel stack top address.
pub fn current_kstack_top() -> usize {
  current_task().unwrap().kstack.get_top()
}

/// Return to idle control flow for new scheduling
pub fn schedule(switched_task_cx_ptr: *mut TaskContext) {
  let mut processor = PROCESSOR.exclusive_access();
  let idle_task_cx_ptr = processor.get_idle_task_cx_ptr();
  drop(processor);
  unsafe {
    __switch(switched_task_cx_ptr, idle_task_cx_ptr);
  }
}
