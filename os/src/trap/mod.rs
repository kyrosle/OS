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
    check_signals_error_of_current, current_add_signal,
    current_trap_cx, current_user_token,
    exit_current_and_run_next, handle_signals,
    suspend_current_and_run_next, SignalFlags,
  },
  timer::set_next_trigger,
};

global_asm!(include_str!("trap.S"));

/// initialize CSR `stvec` as the entry of `__alltraps`
pub fn init() {
  set_kernel_trap_entry();
}

/// timer interrupt enabled
pub fn enable_timer_interrupt() {
  unsafe {
    sie::set_stimer();
  }
}

fn set_kernel_trap_entry() {
  unsafe {
    stvec::write(
      trap_from_kernel as usize,
      TrapMode::Direct,
    );
  }
}

#[no_mangle]
pub fn trap_from_kernel() -> ! {
  panic!("a trap from kernel!");
}

fn set_user_trap_entry() {
  unsafe {
    stvec::write(TRAMPOLINE, TrapMode::Direct);
  }
}
#[no_mangle]
/// set the new addr of __restore asm function in TRAMPOLINE page,
/// set the reg a0 = trap_cx_ptr, reg a1 = physical address of user page table.
/// finally, jump to new address of __restore asm function.
pub fn trap_return() -> ! {
  set_user_trap_entry();
  let trap_cx_ptr = TRAP_CONTEXT;
  let user_satp = current_user_token();
  extern "C" {
    fn __alltraps();
    fn __restore();
  }
  let restore_va =
    __restore as usize - __alltraps as usize + TRAMPOLINE;
  unsafe {
    asm!(
      "fence.i",
      "jr {restore_va}", // jump to new address of __restore asm function
      restore_va = in(reg) restore_va,
      in("a0") trap_cx_ptr, // a0 = virtual address of TrapContext
      in("a1") user_satp, // a1 = physical page of user page table
      options(noreturn)
    );
  }
}

#[no_mangle]
/// handle an interrupt, exception, or system call from user space.
pub fn trap_handler() -> ! {
  set_kernel_trap_entry();

  let scause = scause::read(); // get trap cause
  let stval = stval::read(); // get extra value

  match scause.cause() {
    Trap::Exception(Exception::UserEnvCall) => {
      // jump to next instruction anywayj
      let mut cx = current_trap_cx();
      cx.sepc += 4;
      // get system call return value
      let result =
        syscall(cx.x[17], [cx.x[10], cx.x[11], cx.x[12]])
          as usize;
      // cx is changed during sys_exec, so we have to call it again
      cx = current_trap_cx();
      cx.x[10] = result;
    }
    Trap::Exception(Exception::StoreFault)
    | Trap::Exception(Exception::StorePageFault)
    | Trap::Exception(Exception::InstructionFault)
    | Trap::Exception(Exception::InstructionPageFault)
    | Trap::Exception(Exception::LoadFault)
    | Trap::Exception(Exception::LoadPageFault) => {
      // println!(
      //     "[kernel] {:?} in application, bad addr = {:#x}, bad instruction = {:#x}, kernel killed it.",
      //     scause.cause(),
      //     stval,
      //     current_trap_cx().sepc,
      // );
      // page fault exit code
      // exit_current_and_run_next(-2);
      current_add_signal(SignalFlags::SIGSEGV);
    }
    Trap::Exception(Exception::IllegalInstruction) => {
      // println!("[kernel] IllegalInstruction in application, kernel killed it");
      // exit_current_and_run_next(-3);
      current_add_signal(SignalFlags::SIGILL);
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
  handle_signals();

  // check error signals (if error then exit)
  if let Some((errno, msg)) =
    check_signals_error_of_current()
  {
    println!("[kernel] {}", msg);
    exit_current_and_run_next(errno);
  }
  trap_return();
}
