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

const USER_HEAP_SIZE: usize = 16384;

static mut HEAP_SPACE: [u8; USER_HEAP_SIZE] =
  [0; USER_HEAP_SIZE];

#[global_allocator]
static HEAP: LockedHeap<32> = LockedHeap::empty();

#[alloc_error_handler]
pub fn handler_alloc_error(
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

/// ### Function:
///     Copy an already open file in the process and assign it to a new file descriptor.
///
/// ### Parameter:
///   - `fd` a file descriptor represent a already open file.
///
/// ### Return value:
///     if the accepting address is invalid, return -1, otherwise return 0.
///
/// syscall ID: 24
pub fn dup(fd: usize) -> isize {
  sys_dup(fd)
}

/// ### Function:
///     Open a pipeline for the current process.
///
/// ### Parameter:
///   - `pipe` represents a `usize` array starting address in application address space with the length of 2.
///           kernel should put the file description of read-end and write-end into this array.
///
/// ### Return value:
///     if the accepting address is invalid, return -1, otherwise return 0.
///
/// syscall ID: 59
pub fn pipe(pipe_fd: &mut [usize]) -> isize {
  sys_pipe(pipe_fd)
}

/// ### Function:
///     Open a regular file and return a file descriptor that can access it.
///
/// ### Parameter:
///   - `path` represents the name of file wanna to open.
///   - `flags` flags describing(as fellow) open files.
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
  sys_open(path, flags.bits())
}

/// ### Function:
///   Close a file in current process.
///
/// ### Parameter:
///   - `fd` the file descriptor should be closed.
///
/// ### Return:
///   if the closing success return 0, otherwise return -1, cos of the file descriptor doesn't match a opening file.
pub fn close(fd: usize) -> isize {
  sys_close(fd)
}

/// ### Function:
///   Read the data to the buffer in memory from a file.
///
/// ### Parameter:
///   - `fd` represents the file descriptor of the file to be read;
///   - `buf` represents the starting address of the buffer in memory(fat pointer, containing the start of the buffer address and the buffer size);
///   - `len` indicates the length of the buffer in memory.
///
/// ### Return value:
///   Returns the length of a successful read.
///
/// syscall ID: 64
pub fn read(fd: usize, buf: &mut [u8]) -> isize {
  sys_read(fd, buf)
}

/// ### Function:
///   Write the data in the buffer in memory to a file.
///
/// ### Parameter:
///   - `fd` represents the file descriptor of the file to be written;
///   - `buf` represents the starting address of the buffer in memory(fat pointer, containing the start of the buffer address and the buffer size);
///   - `len` indicates the length of the buffer in memory.
///
/// ### Return value:
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
/// ### Parameter:
///   `exit_code` represents the return value of the application.
/// ### Return value:
///   This system call should not return.
///
/// syscall ID: 93
pub fn exit(exit_code: i32) -> ! {
  sys_exit(exit_code);
}

/// ### Function:
///   Indicates that the application itself `temporarily` gives up the current right to use the CPU and enters the `Ready` state.
/// ### Return value:
///   Returns whether the execution was successful, and returns 0 if successful.
///
/// syscall ID: 93
pub fn yield_() -> isize {
  sys_yield()
}

/// ### Function:
///   Get the current time, saved in the TimeVal struct ts, _tz ignored in our implementation.
/// ### Return value:
///   Returns whether the execution was successful, and returns 0 if successful.
///
/// syscall ID: 169
pub fn get_time() -> isize {
  sys_get_time()
}

/// ### Function:
///   Get the current process pid.
/// ### Return value:
///   Returns current process pid.
///
/// syscall ID: 172
pub fn getpid() -> isize {
  sys_getpid()
}

/// ### Function:
///   The current process forks out a child process.
/// ### Return value:
///   Returns 0 for the child process, and returns the PID of the child process for the current process.
///
/// syscall ID: 220
pub fn fork() -> isize {
  sys_fork()
}

/// ### Function:
///   Empty the address space of the current process and load a specific executable file,
///     return to user mode and start its execution.
/// ### Parameter:
///   - path The name of the executable to load.
///   - args the elements of this array are the start address of each parameter strings
/// ### Return value:
///   Returns -1 if there is an error (if no executable matching the name is found),
///     otherwise it should not be returned
///
/// syscall ID: 221
pub fn exec(path: &str, args: &[*const u8]) -> isize {
  sys_exec(path, args)
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
///   The current process waits for a child process to become a zombie process,
///   reclaiming all its resources and collecting its return value.
/// ### Parameters:
///   - pid: Represents the process ID of the child process to wait,
///           if -1, it means waiting for any child process.
///   - exit_code: Indicates the address to save the return value of the child process.
///           If this address is 0, it means that it does not need to be saved.
/// ### Return value:
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

pub fn sleep(period_ms: usize) {
  let start = sys_get_time();
  while sys_get_time() < start + period_ms as isize {
    sys_yield();
  }
}

#[repr(C, align(16))]
#[derive(Debug, Clone, Copy)]
/// We align it to 16 bytes so that it doesn't cross virtual pages.
///
/// [Mention]: It should be noted that our current implementation is relatively simple
/// and does not support signal nesting for the time being,
/// that is, to execute another signal processing routine
/// during the execution of one signal processing routine.
///
/// (same as os/task/action/SignalAction)
pub struct SignalAction {
  /// Represents the entry address of the signal processing routine.
  pub handler: usize,
  /// Indicates the signal `mask` during execution of the signal processing routine.
  pub mask: SignalFlags,
}

impl Default for SignalAction {
  fn default() -> Self {
    Self {
      handler: 0,
      mask: SignalFlags::empty(),
    }
  }
}

pub const SIGDEF: i32 = 0; // Default signal handling
pub const SIGHUP: i32 = 1;
pub const SIGINT: i32 = 2;
pub const SIGQUIT: i32 = 3;
pub const SIGILL: i32 = 4;
pub const SIGTRAP: i32 = 5;
pub const SIGABRT: i32 = 6;
pub const SIGBUS: i32 = 7;
pub const SIGFPE: i32 = 8;
pub const SIGKILL: i32 = 9;
pub const SIGUSR1: i32 = 10;
pub const SIGSEGV: i32 = 11;
pub const SIGUSR2: i32 = 12;
pub const SIGPIPE: i32 = 13;
pub const SIGALRM: i32 = 14;
pub const SIGTERM: i32 = 15;
pub const SIGSTKFLT: i32 = 16;
pub const SIGCHLD: i32 = 17;
pub const SIGCONT: i32 = 18;
pub const SIGSTOP: i32 = 19;
pub const SIGTSTP: i32 = 20;
pub const SIGTTIN: i32 = 21;
pub const SIGTTOU: i32 = 22;
pub const SIGURG: i32 = 23;
pub const SIGXCPU: i32 = 24;
pub const SIGXFSZ: i32 = 25;
pub const SIGVTALRM: i32 = 26;
pub const SIGPROF: i32 = 27;
pub const SIGWINCH: i32 = 28;
pub const SIGIO: i32 = 29;
pub const SIGPWR: i32 = 30;
pub const SIGSYS: i32 = 31;

bitflags! {
  #[derive(Clone, Copy, Debug)]
  pub struct SignalFlags: i32 {
    const SIGDEF = 1; // Default signal handling
    const SIGHUP = 1 << 1;
    const SIGINT = 1 << 2;
    const SIGQUIT = 1 << 3;
    const SIGILL = 1 << 4;
    const SIGTRAP = 1 << 5;
    const SIGABRT = 1 << 6;
    const SIGBUS = 1 << 7;
    const SIGFPE = 1 << 8;
    const SIGKILL = 1 << 9;
    const SIGUSR1 = 1 << 10;
    const SIGSEGV = 1 << 11;
    const SIGUSR2 = 1 << 12;
    const SIGPIPE = 1 << 13;
    const SIGALRM = 1 << 14;
    const SIGTERM = 1 << 15;
    const SIGSTKFLT = 1 << 16;
    const SIGCHLD = 1 << 17;
    const SIGCONT = 1 << 18;
    const SIGSTOP = 1 << 19;
    const SIGTSTP = 1 << 20;
    const SIGTTIN = 1 << 21;
    const SIGTTOU = 1 << 22;
    const SIGURG = 1 << 23;
    const SIGXCPU = 1 << 24;
    const SIGXFSZ = 1 << 25;
    const SIGVTALRM = 1 << 26;
    const SIGPROF = 1 << 27;
    const SIGWINCH = 1 << 28;
    const SIGIO = 1 << 29;
    const SIGPWR = 1 << 30;
    const SIGSYS = 1 << 31;
  }
}

/// ### Function:
///   current process send a signal to another process(may be itself).
/// ### Parameters:
///   - pid the acceptor process ID
///   - signum the number of signal
/// ### Return value:
///   Return -1 if the parameters is invalid(the specified process or signal type does not exist), otherwise return 0.
///
/// syscall ID: 129
pub fn kill(pid: usize, signum: i32) -> isize {
  sys_kill(pid, signum)
}

/// ### Function:
///   Setting a handler for specified signal in current process, meanwhile store the previous handler function.
/// ### Parameters:
///   - signum the number of signal
///   - action the handler want set
///   - old_action store the previous handler
/// ### Return value:
///   return -1 if the `action` or `old_action` is nullptr, or the signum is invalid, otherwise return 0.
///
/// syscall ID: 134
pub fn sigaction(
  signum: i32,
  action: Option<&SignalAction>,
  old_action: Option<&mut SignalAction>,
) -> isize {
  sys_sigaction(
    signum,
    action.map_or(core::ptr::null(), |a| a),
    old_action.map_or(core::ptr::null_mut(), |a| a),
  )
}

/// ### Function:
///   setting the global signal mask in current process.
/// ### Parameter:
///   mask represents the global signal mask will be set, representing
///   a set of signals(all signals in this set will be always ignored by current process).
/// ### Return value:
///   if the parameter is invalid return -1, other return the previous mask.
///
/// syscall ID: 135
pub fn sigprocmask(mask: u32) -> isize {
  sys_sigprocmask(mask)
}

/// ### Function:
///   The process notifies the kernel that the signal processing routine exits,
///     and the original process execution can be resumed.
/// ### Return value:
///   if error happened return -1, otherwise return 0.
pub fn sigreturn() -> isize {
  sys_sigreturn()
}
