use core::arch::asm;

const SYSCALL_WRITE: usize = 64;
const SYSCALL_EXIT: usize = 93;
const SYSCALL_YIELD: usize = 124;
const SYSCALL_GET_TIME: usize = 169;

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

pub fn sys_yield() -> isize {
  syscall(SYSCALL_YIELD, [0, 0, 0])
}

pub fn sys_get_time() -> isize {
  syscall(SYSCALL_GET_TIME, [0, 0, 0])
}
