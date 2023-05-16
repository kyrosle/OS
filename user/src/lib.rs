#![no_std]
#![feature(linkage)]
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]

#[macro_use]
pub mod console;
mod lang_items;
mod syscall;

extern crate alloc;
#[macro_use]
extern crate bitflags;

use alloc::vec::Vec;
use buddy_system_allocator::LockedHeap;
use syscall::*;

const USER_HEAP_SIZE: usize = 32768;

static mut HEAP_SPACE: [u8; USER_HEAP_SIZE] =
  [0; USER_HEAP_SIZE];

#[global_allocator]
static HEAP: LockedHeap<32> = LockedHeap::empty();

#[alloc_error_handler]
pub fn handle_alloc_error(
  layout: core::alloc::Layout,
) -> ! {
  panic!("Heap allocation error, layout = {:?}", layout);
}

#[no_mangle]
#[link_section = ".text.entry"]
pub extern "C" fn _start(argc: usize, argv: usize) -> ! {
  unsafe {
    HEAP
      .lock()
      .init(HEAP_SPACE.as_ptr() as usize, USER_HEAP_SIZE);
  }
  //take out the starting addresses of argc strings respectively.
  let mut v: Vec<&'static str> = Vec::new();
  for i in 0..argc {
    let str_start = unsafe {
      ((argv + i * core::mem::size_of::<usize>())
        as *const usize)
        .read_volatile()
    };
    let len = (0usize..)
      .find(|i| unsafe {
        ((str_start + *i) as *const u8).read_volatile() == 0
      })
      .unwrap();
    v.push(
      core::str::from_utf8(unsafe {
        core::slice::from_raw_parts(
          str_start as *const u8,
          len,
        )
      })
      .unwrap(),
    )
  }
  exit(main(argc, v.as_slice()));
}

#[linkage = "weak"]
#[no_mangle]
fn main(_argc: usize, _argv: &[&str]) -> i32 {
  panic!("Cannot find main!");
}

bitflags! {
  pub struct OpenFlags: u32 {
    const RDONLY = 0;
    const WRONLY = 1 << 0;
    const RDWR = 1 << 1;
    const CREATE = 1 << 9;
    const TRUNC = 1 << 10;
  }
}

bitflags! {
  pub struct SignalFlags: i32 {
    const SIGINT    = 1 << 2;
    const SIGILL    = 1 << 4;
    const SIGABRT   = 1 << 6;
    const SIGFPE    = 1 << 8;
    const SIGSEGV   = 1 << 11;
  }
}

/// ### Function:
///     Copy an already open file in the process and assign it to a new file descriptor.
///
/// ### Parameter:
///   - `fd`: a file descriptor represent a already open file.
///
/// ### Return value:
///     if the accepting address is invalid, return -1, otherwise return 0.
///
/// syscall ID: 24
pub fn dup(fd: usize) -> isize {
  sys_dup(fd)
}

/// ### Function:
///     Open a regular file and return a file descriptor that can access it.
///
/// ### Parameter:
///   - `path`: represents the name of file wanna to open.
///   - `flags`: flags describing(as fellow) open files.
///
/// | Flags | Value  | File ModeDescription                                                                |
/// | ----- | ------ | ----------------------------------------------------------------------------------- |
/// | 0     | RDONLY | File is opened in read-only mode                                                    |
/// | 0x001 | WRONLY | File is opened in write-only mode                                                   |
/// | 0x002 | RDWR   | File is opened in read-write mode                                                   |
/// | 0x200 | CREATE | File is created if it does not exist, and its size is set to 0 if it already exists |
/// | 0x400 | TRUNC  | File is opened with its contents cleared and its size set to 0                      |
///
/// ### Return value:
///   Returns a file descriptor if success, otherwise will return -1 cos of the file may not exist.
///
/// syscall ID: 56
pub fn open(path: &str, flags: OpenFlags) -> isize {
  sys_open(path, flags.bits)
}

/// ### Function:
///   Close a file in current process.
///
/// ### Parameter:
///   - `fd`: the file descriptor should be closed.
///
/// ### Return:
///   if the closing success return 0, otherwise return -1, cos of the file descriptor doesn't match a opening file.
///
/// syscall ID: 57
pub fn close(fd: usize) -> isize {
  sys_close(fd)
}

/// ### Function:
///     Open a pipeline for the current process.
///
/// ### Parameter:
///   - `pipe`: represents a `usize` array starting address in application address space with the length of 2.
///           kernel should put the file description of read-end and write-end into this array.
///
/// ### Return:
///     if the accepting address is invalid, return -1, otherwise return 0.
///
/// syscall ID: 59
pub fn pipe(pipe_fd: &mut [usize]) -> isize {
  sys_pipe(pipe_fd)
}

/// ### Function:
///   Read the data to the buffer in memory from a file.
///
/// ### Parameters:
///   - `fd`: represents the file descriptor of the file to be read;
///   - `buf`: represents the starting address of the buffer in memory(fat pointer, containing the start of the buffer address and the buffer size);
///   - `len`: indicates the length of the buffer in memory.
///
/// ### Return value:
///   Returns the length of a successful read.
///
/// syscall ID: 63
pub fn read(fd: usize, buf: &mut [u8]) -> isize {
  sys_read(fd, buf)
}

/// ### Function:
///   Write the data in the buffer in memory to a file.
///
/// ### Parameters:
///   - `fd`: represents the file descriptor of the file to be written;
///   - `buf`: represents the starting address of the buffer in memory(fat pointer, containing the start of the buffer address and the buffer size);
///   - `len`: indicates the length of the buffer in memory.
///
/// ### Return:
///   Returns the length of a successful write.
///
/// syscall ID: 64
///
/// fd set as `1`, meaning that it's a standard output(that is output to the screen).
pub fn write(fd: usize, buf: &[u8]) -> isize {
  sys_write(fd, buf)
}

/// ### Function:
///   Exit the application and inform the batch system of the return value.
///
/// ### Parameter:
///   - `exit_code`: represents the return value of the application.
///
/// ### Return:
///   This system call should not return.
///
/// syscall ID: 93
pub fn exit(exit_code: i32) -> ! {
  sys_exit(exit_code);
}

/// ### Function:
///   current thread sleep for `period_ms` milliseconds.
///
/// ### Parameter:
///   - `sleep_ms`: the milliseconds wanted to sleep.
///
/// syscall ID: 101
pub fn sleep(sleep_ms: usize) {
  sys_sleep(sleep_ms);
}

/// ### Function:
///   Indicates that the application itself `temporarily` gives up the current
///   right to use the CPU and enters the `Ready` state.
///
/// ### Return:
///   Returns whether the execution was successful, and returns 0 if successful.
///
/// syscall ID: 124
pub fn yield_() -> isize {
  sys_yield()
}

/// ### Function:
///   current process send a signal to another process(may be itself).
///
/// ### Parameters:
///   - `pid`: the acceptor process ID
///   - `signum`: the number of signal
///
/// ### Return:
///   Return -1 if the parameters is invalid(the specified process or signal type does not exist), otherwise return 0.
///
/// syscall ID: 129
pub fn kill(pid: usize, signum: i32) -> isize {
  sys_kill(pid, signum)
}

/// ### Function:
///   Get the current time, saved in the TimeVal struct ts, _tz ignored in our implementation.
///
/// ### Return:
///   Returns whether the execution was successful, and returns 0 if successful.
///
/// syscall ID: 169
pub fn get_time() -> isize {
  sys_get_time()
}

/// ### Function:
///   Get the current process pid.
///
/// ### Return:
///   Returns current process pid.
///
/// syscall ID: 172
pub fn getpid() -> isize {
  sys_getpid()
}

/// ### Function:
///   The current process forks out a child process.
///
/// ### Return:
///   Returns 0 for the child process, and returns the PID of the child process for the current process.
///
/// syscall ID: 220
pub fn fork() -> isize {
  sys_fork()
}

/// ### Function:
///     Empty the address space of the current process and load a specific executable file,
///     return to user mode and start its execution.
///
/// ### Parameter:
///   - `path`: The name of the executable to load.
///   - `args`: the elements of this array are the start address of each parameter strings
///
/// ### Return:
///   Returns -1 if there is an error (if no executable matching the name is found),
///     otherwise it should not be returned
///
/// syscall ID: 221
pub fn exec(path: &str, args: &[*const u8]) -> isize {
  sys_exec(path, args)
}

/// ### Function:
///   The current process waits for a child process to become a zombie process,
///   reclaiming all its resources and collecting its return value.
///
/// ### Parameters:
///   - `pid`: Represents the process ID of the child process to wait,
///           if -1, it means waiting for any child process.
///   - `exit_code`: Indicates the address to save the return value of the child process.
///           If this address is 0, it means that it does not need to be saved.
/// ### Return:
///   Returns -1 if the child process to wait for does not exist;
///     Otherwise returns -2 if none of the child processes to wait for have ended.
///     Otherwise return the process ID of the ended child process
///
/// syscall ID: 260
pub fn waitpid(pid: usize, exit_code: &mut i32) -> isize {
  loop {
    match sys_waitpid(pid as isize, exit_code as *mut _) {
      -2 => {
        yield_();
      }
      // -1 or a real pid
      exit_pid => return exit_pid,
    }
  }
}

pub fn wait(exit_code: &mut i32) -> isize {
  loop {
    match sys_waitpid(-1, exit_code as *mut _) {
      -2 => {
        yield_();
      }
      // -1 or a real pid
      exit_pid => return exit_pid,
    }
  }
}

pub fn waitpid_nb(
  pid: usize,
  exit_code: &mut i32,
) -> isize {
  sys_waitpid(pid as isize, exit_code as *mut _)
}

/// ### Function:
///   create a new thread in current process
///
/// ### Parameters:
///   - `entry`: the thread function entry point.
///   - `arg`: the params provided to thread
///
/// ### Return:
///   the tid of created thread.
///
/// syscall ID: 1000
pub fn thread_create(entry: usize, arg: usize) -> isize {
  sys_thread_create(entry, arg)
}

/// ### Function:
///   Get the current thread tid
///
/// ### Return:
///   return the current thread tid.
///
/// syscall ID: 1001
pub fn gettid() -> isize {
  sys_gettid()
}

/// ### Function:
///   waiting a thread in current process.
///
/// ### Parameter:
///   - `tid`: represent the tid of the specified thread.
///
/// ### Return:
///   If the thread is not existed, return -1,
///   if the thread haven't not exited, return -2,
///   otherwise, return the exit_code of this thread.
///
/// syscall ID: 1002
pub fn waittid(tid: usize) -> isize {
  loop {
    match sys_waittid(tid) {
      -2 => {
        yield_();
      }
      exit_code => return exit_code,
    }
  }
}

/// ### Function:
///   create a new mutex in current process.
///
/// ### Parameter:
///   - `blocking`: if setting `true`, representing this mutex is used in blocking.
///
/// ### Return:
///   Assuming this operation always succeed, and return the mutex id.
///
/// syscall ID: 1010
pub fn mutex_create() -> isize {
  sys_mutex_create(false)
}

/// ### Function:
///   create a new `blocking` mutex in current process.
///
/// ### Parameter:
///   - `blocking`: if setting `true`, representing this mutex is used in blocking.
///
/// ### Return:
///   Assuming this operation always succeed, and return the mutex id.
///
/// syscall ID: 1010
pub fn mutex_blocking_create() -> isize {
  sys_mutex_create(true)
}

/// ### Function:
///   try to acquire the mutex in current process.
///
/// ### Parameter:
///   - `mutex_id`: represent the id of the mutex to acquire.
///
/// ### Return:
///   return 0.
///
/// syscall ID: 1011
pub fn mutex_lock(mutex_id: usize) -> isize {
  sys_mutex_lock(mutex_id)
}

/// ### Function:
///   release a mutex in current process.
///
/// ### Parameter:
///   - `mutex_id`: represent the id of mutex to release.
///
/// ### Return:
///   return 0.
///
/// syscall ID: 1012
pub fn mutex_unlock(mutex_id: usize) -> isize {
  sys_mutex_unlock(mutex_id)
}

/// ### Function:
///   create a semaphore in current process.
///
/// ### Parameter:
///   - `res_count`: Indicates the initial
///         resource available quantity for this semaphore, a usize.
///
/// ### Return:
///   assuming this operation always succeed, return the semaphore id.
///
/// syscall ID: 1020
pub fn semaphore_create(res_count: usize) -> isize {
  sys_semaphore_create(res_count)
}

/// ### Function:
///   doing the V operation in specified semaphore in current process.
///
/// ### Parameter:
///   - `sem_id`: the specified semaphore id.
///
/// ### Return:
///   assuming this operation always succeed, return 0.
///
/// syscall ID: 1021
pub fn semaphore_up(sem_id: usize) -> isize {
  sys_semaphore_up(sem_id)
}

/// ### Function:
///   doing the P operation in specified semaphore in current process.
///
/// ### Parameter:
///   - `sem_id`: the specified semaphore id.
///
/// ### Return:
///   assuming this operation always succeed, return 0.
///
/// syscall ID: 1022
pub fn semaphore_down(sem_id: usize) -> isize {
  sys_semaphore_down(sem_id)
}

/// ### Function:
///   create a condition variable in current process.
///
/// ### Return:
///   assuming this operation always succeeds, return the condition variable id.
///
/// syscall ID: 1030
pub fn condvar_create() -> isize {
  sys_condvar_create()
}

/// ### Function:
///   Signal operation on the specified condition variable,
///   of the current process, wake up a thread blocking on this condition variable (if present).
///
/// ### Parameter:
///   - `condvar_id`: the speficied condition variable id.
///
/// ### Return:
///   assuming this operation always succeeds, return 0.
///
/// syscall ID: 1031
pub fn condvar_signal(condvar_id: usize) -> isize {
  sys_condvar_signal(condvar_id)
}

/// ### Function:
///   The wait operation on the specified condition variable
///   of the current process is divided into multiple stages:
///   1. release the mutex owning by current thread.
///   2. block the current thread and add it to the blocking queue,
///       with the specified condition variable.
///   3. until the current thread is awakened by another thread
///       through the signal operation.
///
/// ### Parameters:
///   - `mutex_id`: represents the mutex id possessing by current thread.
///   - `condvar_id`: represents the condition variable id.
///
/// ### Return:
///   assuming this operation always succeeds, return 0.
pub fn condvar_wait(
  condvar_id: usize,
  mutex_id: usize,
) -> isize {
  sys_condvar_wait(condvar_id, mutex_id)
}

#[macro_export]
macro_rules! vstore {
  ($var_ref: expr, $value: expr) => {
    unsafe {
      core::intrinsics::volatile_store(
        $var_ref as *const _ as _,
        $value,
      )
    }
  };
}

#[macro_export]
macro_rules! vload {
  ($var_ref: expr) => {
    unsafe {
      core::intrinsics::volatile_load(
        $var_ref as *const _ as _,
      )
    }
  };
}

#[macro_export]
macro_rules! memory_fence {
  () => {
    core::sync::atomic::fence(
      core::sync::atomic::Ordering::SeqCst,
    )
  };
}
