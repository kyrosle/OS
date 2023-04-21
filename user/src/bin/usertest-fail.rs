#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

// not in SUCC_TESTS & FAIL_TESTS
// count_lines, infloop, user_shell, usertests

static FAIL_TESTS: &[(&str, &str, &str, &str, i32)] =
  &[("stack_overflow\0", "\0", "\0", "\0", -2)];

use user_lib::{exec, fork, waitpid};

fn run_tests(tests: &[(&str, &str, &str, &str, i32)]) -> i32 {
  let mut pass_num = 0;
  let mut arr: [*const u8; 4] = [
    core::ptr::null::<u8>(),
    core::ptr::null::<u8>(),
    core::ptr::null::<u8>(),
    core::ptr::null::<u8>(),
  ];

  for test in tests {
    println!("Usertests: Running {}", test.0);
    arr[0] = test.0.as_ptr();
    if test.1 != "\0" {
      arr[1] = test.1.as_ptr();
      arr[2] = core::ptr::null::<u8>();
      arr[3] = core::ptr::null::<u8>();
      if test.2 != "\0" {
        arr[2] = test.2.as_ptr();
        arr[3] = core::ptr::null::<u8>();
        if test.3 != "\0" {
          arr[3] = test.3.as_ptr();
        } else {
          arr[3] = core::ptr::null::<u8>();
        }
      } else {
        arr[2] = core::ptr::null::<u8>();
        arr[3] = core::ptr::null::<u8>();
      }
    } else {
      arr[1] = core::ptr::null::<u8>();
      arr[2] = core::ptr::null::<u8>();
      arr[3] = core::ptr::null::<u8>();
    }
    println!("---- fork {} application ----", test.0);
    let pid = fork();
    if pid == 0 {
      let res = exec(test.0);
      println!("exit code: {}", res);
      panic!("unreachable!");
    } else {
      let mut exit_code: i32 = Default::default();
      let wait_pid = waitpid(pid as usize, &mut exit_code);
      assert_eq!(pid, wait_pid);
      if exit_code == test.4 {
        // summary apps with  exit_code
        pass_num += 1;
      }
      println!(
        "\x1b[32mUsertests: Test {} in Process {} exited with code {}\x1b[0m",
        test.0, pid, exit_code
      );
    }
  }
  pass_num
}

#[no_mangle]
pub fn main() -> i32 {
  let err_num = run_tests(FAIL_TESTS);
  if err_num != FAIL_TESTS.len() as i32 {
    println!(
      "all failed app_num is  {} , but only  passed {}",
      FAIL_TESTS.len(),
      err_num
    );
  }
  println!(" Usertests failed!");
  -1
}
