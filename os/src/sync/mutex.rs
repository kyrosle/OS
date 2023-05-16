use alloc::{collections::VecDeque, sync::Arc};

use crate::task::{
  block_current_and_run_next, current_task,
  suspend_current_and_run_next, wakeup_task,
  TaskControlBlock,
};

use super::UPSafeCell;

pub trait Mutex: Sync + Send {
  fn lock(&self);
  fn unlock(&self);
}

/// ## MutexSpin
///
/// If the mutex is being occupied by another thread,
/// current thread will `yield`, and then current thread(TCB)
/// will add into the manager.ready_queue to waiting for next
/// time being scheduled.
pub struct MutexSpin {
  locked: UPSafeCell<bool>,
}

impl MutexSpin {
  pub fn new() -> Self {
    Self {
      locked: unsafe { UPSafeCell::new(false) },
    }
  }
}

impl Mutex for MutexSpin {
  fn lock(&self) {
    loop {
      let mut locked = self.locked.exclusive_access();
      if *locked {
        drop(locked);
        suspend_current_and_run_next();
        continue;
      } else {
        *locked = true;
        return;
      }
    }
  }

  fn unlock(&self) {
    let mut locked = self.locked.exclusive_access();
    *locked = false;
  }
}

/// ## MutexBlocking
///
/// If the mutex is being occupied by another thread,
/// current thread(TCB) will be marked as ThreadStatus::Blocked,
/// it will not be add into the manager.ready_queue.
///
/// When another thread doing unlock operation and the current thread(TCB)
/// is the head of wait_queue in this mutex, current thread(TCB) will recover the
/// TrapContext and run again.
pub struct MutexBlocking {
  inner: UPSafeCell<MutexBlockingInner>,
}

pub struct MutexBlockingInner {
  locked: bool,
  wait_queue: VecDeque<Arc<TaskControlBlock>>,
}

impl MutexBlocking {
  pub fn new() -> Self {
    Self {
      inner: unsafe {
        UPSafeCell::new(MutexBlockingInner {
          locked: false,
          wait_queue: VecDeque::new(),
        })
      },
    }
  }
}

impl Mutex for MutexBlocking {
  fn lock(&self) {
    let mut mutex_inner = self.inner.exclusive_access();
    if mutex_inner.locked {
      mutex_inner
        .wait_queue
        .push_back(current_task().unwrap());
      drop(mutex_inner);
      block_current_and_run_next();
    } else {
      mutex_inner.locked = true;
    }
  }

  fn unlock(&self) {
    let mut mutex_inner = self.inner.exclusive_access();
    assert!(mutex_inner.locked);
    if let Some(waking_task) =
      mutex_inner.wait_queue.pop_front()
    {
      wakeup_task(waking_task);
    } else {
      mutex_inner.locked = false;
    }
  }
}
