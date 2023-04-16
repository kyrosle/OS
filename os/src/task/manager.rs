//! Implementation of [`TaskManager`]
use alloc::{collections::VecDeque, sync::Arc};

use crate::sync::UPSafeCell;
use lazy_static::lazy_static;

use super::TaskControlBlock;

/// A array of `TaskControlBlock` that is thread-safe
pub struct TaskManager {
  ready_queue: VecDeque<Arc<TaskControlBlock>>,
}

/// A simple FIFO scheduler.
impl TaskManager {
  /// Create an empty TaskManager
  pub fn new() -> Self {
    Self {
      ready_queue: VecDeque::new(),
    }
  }

  /// Add a task to `TaskManager`
  pub fn add(&mut self, task: Arc<TaskControlBlock>) {
    self.ready_queue.push_back(task);
  }

  /// Remove the first task and return it, or `None` if `TaskManager` is empty
  pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
    self.ready_queue.pop_front()
  }
}

lazy_static! {
  pub static ref TASK_MANAGER: UPSafeCell<TaskManager> =
    unsafe { UPSafeCell::new(TaskManager::new()) };
}

/// Interface offered to add task
pub fn add_task(task: Arc<TaskControlBlock>) {
  TASK_MANAGER.exclusive_access().add(task);
}

/// Interface offered to pop the first task
pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {
  TASK_MANAGER.exclusive_access().fetch()
}
