//! Trap handling functionality
//!
//! For rCore, we have a single trap entry point, namely `__alltraps`.
//! At initialization in [`init()`], we set the `stvec` CSR to point to it.
//!
//! All traps go through `__alltraps`, which is defined in `trap.S`.
//! The assembly language code does just enough work restore the kernel
//! space context, ensuring that Rust code safety runs, and transfers control
//! to [`trap_handler()`].
//!
//! It then calls different functionality based on what exactly the exception
//! was. For example, timer interrupts trigger task preemption, and syscalls go
//! to [`syscall()`].

use core::arch::{asm, global_asm};

use riscv::register::{
  mtvec::TrapMode,
  scause::{self, Exception, Interrupt, Trap},
  sie, stval, stvec,
};

pub mod context;
pub use context::TrapContext;

use crate::{
  config::{TRAMPOLINE, TRAP_CONTEXT},
  syscall::syscall,
  task::{
    current_trap_cx, current_user_token, exit_current_and_run_exit,
    suspend_current_and_run_next,
  },
  timer::set_next_trigger,
};

global_asm!(include_str!("trap.S"));

static mut KERNEL_INTERRUPT_TRIGGERED: bool = false;

/// initialize CSR `stvec` as the entry of `__alltraps`
pub fn init() {}

/// timer interrupt enabled
pub fn enable_timer_interrupt() {
  unsafe {
    sie::set_stimer();
  }
}

fn set_kernel_trap_entry() {
  unsafe {
    stvec::write(trap_from_kernel as usize, TrapMode::Direct);
  }
}

#[no_mangle]
pub fn trap_from_kernel() -> ! {
  panic!("a trap from kernel!");
}

fn set_user_trap_entry() {
  unsafe {
    stvec::write(TRAMPOLINE as usize, TrapMode::Direct);
  }
}
#[no_mangle]
pub fn trap_return() -> ! {
  set_kernel_trap_entry();
  let trap_cx_ptr = TRAP_CONTEXT;
  let user_satp = current_user_token();
  extern "C" {
    fn __alltraps();
    fn __restore();
  }
  let restore_va = __restore as usize - __alltraps as usize + TRAMPOLINE;
  unsafe {
    asm!(
      "fence.i",
      "jr {restore_va}",
      restore_va = in(reg) restore_va,
      in("a0") trap_cx_ptr,
      in("a1") user_satp,
      options(noreturn)
    );
  }
  panic!("Unreachable in back_to_user!");
}

/// handle an interrupt, exception, or system call from user space.
#[no_mangle]
pub fn trap_handler(cx: &mut TrapContext) -> &mut TrapContext {
  crate::task::user_time_end();
  let cx = current_trap_cx();
  let scause = scause::read(); // get trap cause
  let stval = stval::read(); // get extra value
  match scause.cause() {
    Trap::Exception(Exception::UserEnvCall) => {
      cx.sepc += 4;
      cx.x[10] = syscall(cx.x[17], [cx.x[10], cx.x[11], cx.x[12]]) as usize;
    }
    Trap::Exception(Exception::StoreFault)
    | Trap::Exception(Exception::StorePageFault) => {
      println!("[kernel] PageFault in application, bad addr = {:#x}, bad instruction = {:#x}, core dumped.", stval, cx.sepc);
      exit_current_and_run_exit();
    }
    Trap::Exception(Exception::IllegalInstruction) => {
      println!("[kernel] IllegalInstruction in application, kernel killed it");
      exit_current_and_run_exit();
    }
    Trap::Interrupt(Interrupt::SupervisorTimer) => {
      set_next_trigger();
      suspend_current_and_run_next();
    }
    _ => {
      panic!(
        "Unsupported trap {:?}, stval = {:#x}",
        scause.cause(),
        stval
      );
    }
  }
  crate::task::user_time_start();
  cx
}
