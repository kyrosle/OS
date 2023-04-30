//! Task management implementation
//!
//! Everything about task management, like starting and switching task is
//! implemented here.
//!
//! A single global instance of [`TaskManager`] called `TASK_MANAGER` controls
//! all the tasks in the operating system.
//!
//! A single global instance of [`Processor`]  called `PROCESSOR` monitors running
//! task(s) for each core.
//!
//! A single global instance of [`PidAllocator`] called `PID_ALLOCATOR` allocates
//! pid for user apps.
//!
//! Be careful when you see `__switch` ASM function in `switch.S`. Control flow around this function
//! might not be what you expect.

use crate::fs::{open_file, OpenFlags};
use crate::sbi::shutdown;
use crate::timer::get_time_ms;
use alloc::sync::Arc;
use lazy_static::lazy_static;

mod context;
mod manager;
mod pid;
mod processor;
mod switch;
#[allow(clippy::module_inception)]
mod task;

pub use context::*;
pub use manager::*;
pub use pid::*;
pub use processor::*;
pub use task::*;

use self::processor::{schedule, task_current_task};

/// the start time of switching.
static mut SWITCH_TIME_START: usize = 0;

/// the total time of switching.
static mut SWITCH_TIME_COUNT: usize = 0;

/// Switching the TaskContext and count the user-time or kernel-time.
unsafe fn __switch(
  current_task_cx_ptr: *mut TaskContext,
  next_task_cx_ptr: *const TaskContext,
) {
  SWITCH_TIME_START = get_time_ms();
  switch::__switch(current_task_cx_ptr, next_task_cx_ptr);
  SWITCH_TIME_COUNT += get_time_ms() - SWITCH_TIME_START;
}

/// Suspend the current `Running` task and run the next task in task list.
pub fn suspend_current_and_run_next() {
  // There must be an application running.
  let task = task_current_task().unwrap();

  // ---- access current TCB exclusively
  let mut task_inner = task.inner_exclusive_access();
  let task_cx_ptr =
    &mut task_inner.task_cx as *mut TaskContext;
  // Change status to Ready
  task_inner.task_status = TaskStatus::Ready;
  drop(task_inner);
  // ---- release current TCB

  // push back to ready queue.
  add_task(task);

  // jump to scheduling cycle
  schedule(task_cx_ptr);
}

/// pid of usertests app in make run TEST=1
pub const IDLE_PID: usize = 0;

pub fn exit_current_and_run_next(exit_code: i32) {
  // take from Processor
  let task = task_current_task().unwrap();

  let pid = task.getpid();
  if pid == IDLE_PID {
    println!(
      "[kernel] Idle process exit witch exit_code {} ...",
      exit_code
    );
    if exit_code != 0 {
      // crate::qemu::QEMU_EXIT_HANDLE.exit_failure();
      shutdown(true);
    } else {
      // crate::qemu::QEMU_EXIT_HANDLE.exit_success();
      shutdown(false);
    }
  }

  // **** access current TCB exclusively
  let mut inner = task.inner_exclusive_access();
  // Change status to Zombie
  inner.task_status = TaskStatus::Zombie;
  // Record exit code
  inner.exit_code = exit_code;
  // do not move to its parent but under initproc

  // ++++ access initproc TCB exclusively
  {
    let mut initproc_inner =
      INITPROC.inner_exclusive_access();
    for child in inner.children.iter() {
      child.inner_exclusive_access().parent =
        Some(Arc::downgrade(&INITPROC));
      initproc_inner.children.push(child.clone());
    }
  }
  // ++++ release parent TCB

  inner.children.clear();
  // deallocate user space
  inner.memory_set.recycle_data_pages();
  drop(inner);
  // **** release current TCB
  // drop task manually to maintain rc correctly
  drop(task);
  // we do not have to save task context
  let mut _unused = TaskContext::zero_init();
  schedule(&mut _unused as *mut _);
}

lazy_static! {
  /// Global process that init user shell
  pub static ref INITPROC: Arc<TaskControlBlock> = Arc::new({
    let inode = open_file("initproc", OpenFlags::RDONLY).unwrap();
    let v = inode.read_all();
    TaskControlBlock::new(v.as_slice())
  });
}

pub fn add_initproc() {
  add_task(INITPROC.clone())
}
