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
use crate::timer::remove_timer;
use alloc::sync::Arc;
use alloc::vec::Vec;
use lazy_static::lazy_static;

mod action;
mod context;
mod id;
mod manager;
mod process;
mod processor;
mod signal;
mod switch;
#[allow(clippy::module_inception)]
mod task;

pub use action::*;
pub use context::*;
pub use id::*;
pub use manager::*;
pub use process::*;
pub use processor::*;
pub use signal::*;
pub use task::*;

use self::processor::{schedule, take_current_task};

/// Suspend the current `Running` task and run the next task in task list.
pub fn suspend_current_and_run_next() {
  // There must be an application running.
  let task = take_current_task().unwrap();

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

pub fn block_current_and_run_next() {
  let task = take_current_task().unwrap();
  let mut task_inner = task.inner_exclusive_access();
  let task_cx_ptr =
    &mut task_inner.task_cx as *mut TaskContext;
  task_inner.task_status = TaskStatus::Blocked;
  drop(task_inner);
  schedule(task_cx_ptr);
}

/// pid of usertests app in make run TEST=1
pub const IDLE_PID: usize = 0;

/// Exit the current `Running` task and run the next task in task list.
pub fn exit_current_and_run_next(exit_code: i32) {
  let task = take_current_task().unwrap();
  let mut task_inner = task.inner_exclusive_access();
  let process = task.process.upgrade().unwrap();
  let tid = task_inner.res.as_ref().unwrap().tid;
  // record exit code
  task_inner.exit_code = Some(exit_code);
  task_inner.res = None;
  // here we do not remove the thread since we are still using the stack
  // it will be deallocated when sys_waittid is called
  drop(task_inner);
  drop(task);
  // however, if this is the main thread of current process
  // the process should terminate at once
  if tid == 0 {
    let pid = process.getpid();
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

    // remove this pid from `PID2PCB`
    remove_from_pid2task(pid);

    let mut process_inner =
      process.inner_exclusive_access();
    // mark this process as a zombie process
    process_inner.is_zombie = true;
    //record exit code of main process
    process_inner.exit_code = exit_code;

    {
      // move all child processes under init process
      let mut initproc_inner =
        INITPROC.inner_exclusive_access();
      for child in process_inner.children.iter() {
        child.inner_exclusive_access().parent =
          Some(Arc::downgrade(&INITPROC));
        initproc_inner.children.push(child.clone());
      }
    }

    // deallocate user resource (including tid/trap_cx/ustack) of all threads
    // it has to be done before we dealloc the whole memory_set
    // otherwise they will be deallocated twice
    let mut recycle_res = Vec::<TaskUserRes>::new();
    for task in
      process_inner.tasks.iter().filter(|t| t.is_some())
    {
      let task = task.as_ref().unwrap();
      // if other tasks are Ready in TaskManger or waiting for a timer to be
      // expired, we should remove them.
      //
      // Mention that we do not need to consider Mutex/Semaphore since they
      // are limited in a single process. Therefore, the blocked tasks are
      // removed when the PCB is deallocated.
      remove_inactive_task(Arc::clone(task));
      let mut task_inner = task.inner_exclusive_access();
      if let Some(res) = task_inner.res.take() {
        recycle_res.push(res);
      }
    }
    // dealloc_tid and dealloc_user_res require access to PCB inner, so we
    // need to collect those user res first, then release process_inner
    // for now to avoid deadlock/double borrow problems.
    drop(process_inner);
    recycle_res.clear();

    let mut process_inner =
      process.inner_exclusive_access();
    process_inner.children.clear();
    // deallocate other data in user space i.e. program code/data section
    process_inner.memory_set.recycle_data_pages();
    // drop file descriptors
    process_inner.fd_table.clear();
    // remove all tasks
    process_inner.tasks.clear();
  }

  drop(process);
  // we do not have to save task context
  let mut _unused = TaskContext::zero_init();
  schedule(&mut _unused as *mut _);
}

lazy_static! {
  /// Global process that init user shell
  pub static ref INITPROC: Arc<ProcessControlBlock> = {
    let inode = open_file("initproc", OpenFlags::RDONLY).unwrap();
    let v = inode.read_all();
    ProcessControlBlock::new(v.as_slice())
  };
}

pub fn add_initproc() {
  let _initproc = INITPROC.clone();
}

pub fn check_signals_of_current(
) -> Option<(i32, &'static str)> {
  let process = current_process();
  let process_inner = process.inner_exclusive_access();
  process_inner.signals.check_error()
}

pub fn current_add_signal(signal: SignalFlags) {
  let process = current_process();
  let mut process_inner = process.inner_exclusive_access();
  process_inner.signals |= signal;
}

pub fn remove_inactive_task(task: Arc<TaskControlBlock>) {
  remove_task(Arc::clone(&task));
  remove_timer(Arc::clone(&task));
}
