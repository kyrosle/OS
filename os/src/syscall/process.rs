//! Process management syscalls
use alloc::{string::String, sync::Arc, vec::Vec};

use crate::{
  fs::{open_file, OpenFlags},
  mm::{translated_ref, translated_refmut, translated_str},
  task::{
    add_task, current_process, current_task,
    current_user_token, exit_current_and_run_next,
    pid2process, suspend_current_and_run_next,
    SignalAction, SignalFlags, MAX_SIG,
  },
  timer::get_time_ms,
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
  pub sec: usize,
  pub usec: usize,
}

/// task exits and submit an exit code.
pub fn sys_exit(exit_code: i32) -> ! {
  exit_current_and_run_next(exit_code);
  panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks.
pub fn sys_yield() -> isize {
  suspend_current_and_run_next();
  0
}

/// get time in milliseconds
pub fn sys_get_time() -> isize {
  get_time_ms() as isize
}

pub fn sys_getpid() -> isize {
  current_task()
    .unwrap()
    .process
    .upgrade()
    .unwrap()
    .getpid() as isize
}

pub fn sys_fork() -> isize {
  let current_process = current_process();
  let new_process = current_process.fork();
  let new_pid = new_process.getpid();
  // modify trap context of new_task, because it returns immediately immediately after switching
  let new_process_inner =
    new_process.inner_exclusive_access();
  let task = new_process_inner.tasks[0].as_ref().unwrap();
  let trap_cx = task.inner_exclusive_access().get_trap_cx();
  // we do not have to move to next instruction since we have done it before
  // for child process, fork returns 0.
  trap_cx.x[10] = 0; // x[10] is a0 register
  new_pid as isize
}

pub fn sys_exec(
  path: *const u8,
  mut args: *const usize,
) -> isize {
  let token = current_user_token();
  let path = translated_str(token, path);

  let mut args_vec: Vec<String> = Vec::new();
  loop {
    let arg_str_ptr = *translated_ref(token, args);
    if arg_str_ptr == 0 {
      break;
    }
    args_vec.push(translated_str(
      token,
      arg_str_ptr as *const u8,
    ));
    unsafe {
      args = args.add(1);
    }
  }

  if let Some(app_inode) =
    open_file(path.as_str(), OpenFlags::RDONLY)
  {
    let all_data = app_inode.read_all();
    let process = current_process();
    let argc = args_vec.len();
    process.exec(all_data.as_slice(), args_vec);
    // return arc because cx.x[10]  will be covered with it later
    argc as isize
  } else {
    -1
  }
}

/// If there is not a child process whose pid is same as given, return -1.
/// Else if there is a child process but it is still running, return -2.
pub fn sys_waitpid(
  pid: isize,
  exit_code_ptr: *mut i32,
) -> isize {
  let process = current_process();
  // find a child process

  let mut inner = process.inner_exclusive_access();
  if !inner
    .children
    .iter()
    .any(|p| pid == -1 || pid as usize == p.getpid())
  {
    return -1;
    // ---- release current PCB
  }
  let pair =
    inner.children.iter().enumerate().find(|(_, p)| {
      // ++++ temporarily access child PCB lock exclusively
      p.inner_exclusive_access().is_zombie
        && (pid == -1 || pid as usize == p.getpid())
      // ++++ release child PCB
    });
  if let Some((idx, _)) = pair {
    let child = inner.children.remove(idx);
    // confirm that child will be deallocated after removing from children list
    assert_eq!(Arc::strong_count(&child), 1);
    let found_pid = child.getpid();
    // ++++ temporarily access child PCB exclusively
    let exit_code =
      child.inner_exclusive_access().exit_code;
    // ++++ release child PCB
    *translated_refmut(
      inner.memory_set.token(),
      exit_code_ptr,
    ) = exit_code;
    found_pid as isize
  } else {
    -2
  }
  // ---- release current PCB lock automatically
}

pub fn sys_kill(pid: usize, signum: i32) -> isize {
  // get the PCB by pid
  // then insert the kill flag into its `signals` field.
  if let Some(process) = pid2process(pid) {
    if let Some(flag) = SignalFlags::from_bits(1 << signum)
    {
      // insert the signal
      process.inner_exclusive_access().signals |= flag;
      0
    } else {
      -1
    }
  } else {
    -1
  }
}
