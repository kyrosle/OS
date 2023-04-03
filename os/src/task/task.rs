use super::TaskContext;

#[derive(Copy, Clone, PartialEq)]
pub enum TaskStatus {
  UnInit,
  Ready,
  Running,
  Exited,
}

#[derive(Copy, Clone)]
pub struct TaskControlBlock {
  pub task_status: TaskStatus,
  pub task_cx: TaskContext,
  pub user_time: usize,
  pub kernel_time: usize,
}
