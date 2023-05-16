//! Implementation of [`TaskManager`]
use alloc::{
  collections::{BTreeMap, VecDeque},
  sync::Arc,
};

use crate::sync::UPSafeCell;
use lazy_static::lazy_static;

use super::{
  ProcessControlBlock, TaskControlBlock, TaskStatus,
};

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

  /// Remove the specified task by matching its reference pointer.
  pub fn remove(&mut self, task: Arc<TaskControlBlock>) {
    if let Some((id, _)) = self
      .ready_queue
      .iter()
      .enumerate()
      .find(|(_, t)| Arc::as_ptr(t) == Arc::as_ptr(&task))
    {
      self.ready_queue.remove(id);
    }
  }
}

lazy_static! {
  pub static ref TASK_MANAGER: UPSafeCell<TaskManager> =
    unsafe { UPSafeCell::new(TaskManager::new()) };
  pub static ref PID2PCB: UPSafeCell<BTreeMap<usize, Arc<ProcessControlBlock>>> =
    unsafe { UPSafeCell::new(BTreeMap::new()) };
}

/// Interface offered to add task(thread).
pub fn add_task(task: Arc<TaskControlBlock>) {
  TASK_MANAGER.exclusive_access().add(task);
}

/// Interface offered to wake up task(thread).
pub fn wakeup_task(task: Arc<TaskControlBlock>) {
  let mut task_inner = task.inner_exclusive_access();
  task_inner.task_status = TaskStatus::Ready;
  drop(task_inner);
  add_task(task);
}

/// Interface offered to remove task(thread) by matching the task reference pointer.
pub fn remove_task(task: Arc<TaskControlBlock>) {
  TASK_MANAGER.exclusive_access().remove(task);
}

/// Interface offered to pop the first task.
pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {
  TASK_MANAGER.exclusive_access().fetch()
}

/// Interface offered to get the task PCB reference by pid.
pub fn pid2process(
  pid: usize,
) -> Option<Arc<ProcessControlBlock>> {
  let map = PID2PCB.exclusive_access();
  map.get(&pid).map(Arc::clone)
}

/// Interface offered to mapping pid to its process.
pub fn insert_into_pid2process(
  pid: usize,
  process: Arc<ProcessControlBlock>,
) {
  PID2PCB.exclusive_access().insert(pid, process);
}

/// Interface offered to remove the task PCB by pid.
pub fn remove_from_pid2task(pid: usize) {
  let mut map = PID2PCB.exclusive_access();
  if map.remove(&pid).is_none() {
    panic!("cannot find pid {} in pid2task!", pid);
  }
}
