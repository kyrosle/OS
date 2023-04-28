use core::arch::asm;

const SYSCALL_OPEN: usize = 56;
const SYSCALL_CLOSE: usize = 57;
const SYSCALL_READ: usize = 63;
const SYSCALL_WRITE: usize = 64;
const SYSCALL_EXIT: usize = 93;
const SYSCALL_YIELD: usize = 124;
const SYSCALL_GET_TIME: usize = 169;
const SYSCALL_GETPID: usize = 172;
const SYSCALL_FORK: usize = 220;
const SYSCALL_EXEC: usize = 221;
const SYSCALL_WAITPID: usize = 260;

fn syscall(id: usize, args: [usize; 3]) -> isize {
  let mut ret: isize;
  unsafe {
    // more details instructions about the macros(`asm!`): https://doc.rust-lang.org/reference/inline-assembly.html
    asm!(
        "ecall",
        // {in_var} => {out_var}
        inlateout("x10") args[0] => ret,
        // It means binding the input parameter `args[1]` to the input register `x11`(a1) of 'ecall'
        in("x11") args[1],
        // Bind input arguments `args[2]` and `id` to input registers `x12`(a2) and `x17`(a7), respectively.
        in("x12") args[2],
        in("x17") id
    );
  }
  ret
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
pub fn sys_open(path: &str, flags: u32) -> isize {
  syscall(SYSCALL_OPEN, [path.as_ptr() as usize, flags as usize, 0])
}

/// ### Function:
///   Close a file in current process.
///
/// ### Parameter:
///   - `fd` the file descriptor should be closed.
///
/// ### Return:
///   if the closing success return 0, otherwise return -1, cos of the file descriptor doesn't match a opening file.
pub fn sys_close(fd: usize) -> isize {
  syscall(SYSCALL_CLOSE, [fd, 0, 0])
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
pub fn sys_read(fd: usize, buffer: &mut [u8]) -> isize {
  syscall(
    SYSCALL_READ,
    [fd, buffer.as_mut_ptr() as usize, buffer.len()],
  )
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
pub fn sys_write(fd: usize, buffer: &[u8]) -> isize {
  syscall(SYSCALL_WRITE, [fd, buffer.as_ptr() as usize, buffer.len()])
}

/// ### Function:
///   Exit the application and inform the batch system of the return value.
/// ### Parameter:
///   `exit_code` represents the return value of the application.
/// ### Return value:
///   This system call should not return.
///
/// syscall ID: 93
pub fn sys_exit(exit_code: i32) -> isize {
  syscall(SYSCALL_EXIT, [exit_code as usize, 0, 0])
}

/// ### Function:
///   Indicates that the application itself `temporarily` gives up the current right to use the CPU and enters the `Ready` state.
/// ### Return value:
///   Returns whether the execution was successful, and returns 0 if successful.
///
/// syscall ID: 93
pub fn sys_yield() -> isize {
  syscall(SYSCALL_YIELD, [0, 0, 0])
}

/// ### Function:
///   Get the current time, saved in the TimeVal struct ts, _tz ignored in our implementation.
/// ### Return value:
///   Returns whether the execution was successful, and returns 0 if successful.
///
/// syscall ID: 169
pub fn sys_get_time() -> isize {
  syscall(SYSCALL_GET_TIME, [0, 0, 0])
}

/// ### Function:
///   Get the current process pid.
/// ### Return value:
///   Returns current process pid.
///
/// syscall ID: 172
pub fn sys_getpid() -> isize {
  syscall(SYSCALL_GETPID, [0, 0, 0])
}

/// ### Function:
///   The current process forks out a child process.
/// ### Return value:
///   Returns 0 for the child process, and returns the PID of the child process for the current process.
///
/// syscall ID: 220
pub fn sys_fork() -> isize {
  syscall(SYSCALL_FORK, [0, 0, 0])
}

/// ### Function:
///   Empty the address space of the current process and load a specific executable file,
///     return to user mode and start its execution.
/// ### Parameter:
///   path:  The name of the executable to load.
/// ### Return value:
///   Returns -1 if there is an error (if no executable matching the name is found),
///     otherwise it should not be returned
///
/// syscall ID: 221
pub fn sys_exec(path: &str) -> isize {
  syscall(SYSCALL_EXEC, [path.as_ptr() as usize, 0, 0])
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
pub fn sys_waitpid(pid: isize, exit_code: *mut i32) -> isize {
  syscall(SYSCALL_WAITPID, [pid as usize, exit_code as usize, 0])
}
