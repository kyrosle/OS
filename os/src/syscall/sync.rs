use alloc::sync::Arc;

use crate::{
  sync::{Mutex, MutexBlocking, MutexSpin},
  task::{
    block_current_and_run_next, current_process,
    current_task,
  },
  timer::{add_timer, get_time_ms},
};

pub fn sys_sleep(ms: usize) -> isize {
  let expire_ms = get_time_ms() + ms;
  let task = current_task().unwrap();
  add_timer(expire_ms, task);
  block_current_and_run_next();
  0
}

pub fn sys_mutex_create(blocking: bool) -> isize {
  let process = current_process();
  let mutex: Option<Arc<dyn Mutex>> = if !blocking {
    Some(Arc::new(MutexSpin::new()))
  } else {
    Some(Arc::new(MutexBlocking::new()))
  };

  let mut process_inner = process.inner_exclusive_access();
  if let Some(id) = process_inner
    .mutex_list
    .iter()
    .enumerate()
    .find(|(_, item)| item.is_none())
    .map(|(id, _)| id)
  {
    process_inner.mutex_list[id] = mutex;
    id as isize
  } else {
    process_inner.mutex_list.push(mutex);
    process_inner.mutex_list.len() as isize - 1
  }
}

pub fn sys_mutex_lock(mutex_id: usize) -> isize {
  let process = current_process();
  let process_inner = process.inner_exclusive_access();
  let mutex = Arc::clone(
    process_inner.mutex_list[mutex_id].as_ref().unwrap(),
  );
  drop(process_inner);
  drop(process);
  mutex.lock();
  0
}

pub fn sys_mutex_unlock(mutex_id: usize) -> isize {
  let process = current_process();
  let process_inner = process.inner_exclusive_access();
  let mutex = Arc::clone(
    process_inner.mutex_list[mutex_id].as_ref().unwrap(),
  );
  drop(process_inner);
  drop(process);
  mutex.unlock();
  0
}
