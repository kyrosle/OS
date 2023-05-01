//! Types related to task management
use core::cell::RefMut;

use alloc::string::String;
use alloc::vec;
use alloc::{
  sync::{Arc, Weak},
  vec::Vec,
};

use crate::fs::{Stdin, Stdout};
use crate::mm::translated_refmut;
use crate::{
  config::TRAP_CONTEXT,
  fs::File,
  mm::{MemorySet, PhysPageNum, VirtAddr, KERNEL_SPACE},
  sync::UPSafeCell,
  trap::{trap_handler, TrapContext},
};

use super::{
  pid_alloc, KernelStack, PidHandler, SignalActions,
  SignalFlags, TaskContext,
};

#[derive(Copy, Clone, PartialEq)]
pub enum TaskStatus {
  Ready,
  Running,
  Zombie,
}

pub struct TaskControlBlock {
  // immutable
  pub pid: PidHandler,
  pub kernel_stack: KernelStack,

  // mutable
  inner: UPSafeCell<TaskControlBlockInner>,
}

impl TaskControlBlock {
  pub fn inner_exclusive_access(
    &self,
  ) -> RefMut<'_, TaskControlBlockInner> {
    self.inner.exclusive_access()
  }

  pub fn new(elf_data: &[u8]) -> Self {
    // memory_set with elf program headers/trampoline/trap-context/user-stack
    let (memory_set, user_sp, entry_point) =
      MemorySet::from_elf(elf_data);
    let trap_cx_ppn = memory_set
      .translate(VirtAddr::from(TRAP_CONTEXT).into())
      .unwrap()
      .ppn();

    // alloc a pid and a kernel stack in kernel space
    let pid_handle = pid_alloc();
    let kernel_stack = KernelStack::new(&pid_handle);
    let kernel_stack_top = kernel_stack.get_top();

    let task_control_block = Self {
      pid: pid_handle,
      kernel_stack,
      inner: unsafe {
        UPSafeCell::new(TaskControlBlockInner {
          trap_cx_ppn,
          base_size: user_sp,
          task_cx: TaskContext::goto_trap_return(
            kernel_stack_top,
          ),
          task_status: TaskStatus::Ready,
          memory_set,
          parent: None,
          children: Vec::new(),
          exit_code: 0,
          fd_table: vec![
            // 0 -> stdin
            Some(Arc::new(Stdin)),
            // 1 -> stdout
            Some(Arc::new(Stdout)),
            // 2 -> stderr
            Some(Arc::new(Stdout)),
          ],
          signals: SignalFlags::empty(),
          signal_mask: SignalFlags::empty(),
          handling_sig: -1,
          signal_actions: SignalActions::default(),
          killed: false,
          frozen: false,
          trap_ctx_backup: None,
          user_time: 0,
          kernel_time: 0,
        })
      },
    };

    // prepare TrapContext in user space
    let trap_cx = task_control_block
      .inner_exclusive_access()
      .get_trap_cx();
    *trap_cx = TrapContext::app_init_context(
      entry_point,
      user_sp,
      KERNEL_SPACE.exclusive_access().token(),
      kernel_stack_top,
      trap_handler as usize,
    );
    task_control_block
  }

  pub fn exec(&self, elf_data: &[u8], args: Vec<String>) {
    // memory_set with elf program headers/trampoline/trap context/user stack
    let (memory_set, mut user_sp, entry_point) =
      MemorySet::from_elf(elf_data);
    let trap_cx_ppn = memory_set
      .translate(VirtAddr::from(TRAP_CONTEXT).into())
      .unwrap()
      .ppn();

    // push arguments on user stack
    user_sp -=
      (args.len() + 1) * core::mem::size_of::<usize>();
    let argv_base = user_sp;
    let mut argv = (0..=args.len())
      .map(|arg| {
        translated_refmut(
          memory_set.token(),
          (argv_base + arg * core::mem::size_of::<usize>())
            as *mut usize,
        )
      })
      .collect::<Vec<_>>();
    *argv[args.len()] = 0;
    for i in 0..args.len() {
      user_sp -= args[i].len() + 1;
      *argv[i] = user_sp;
      let mut p = user_sp;
      for c in args[i].as_bytes() {
        *translated_refmut(
          memory_set.token(),
          p as *mut u8,
        ) = *c;
        p += 1;
      }
      *translated_refmut(
        memory_set.token(),
        p as *mut u8,
      ) = 0;
    }
    user_sp -= user_sp % core::mem::size_of::<usize>();

    // **** access inner exclusively
    let mut inner = self.inner_exclusive_access();
    // substitute memory_set
    inner.memory_set = memory_set;
    // update trap_cx ppn
    inner.trap_cx_ppn = trap_cx_ppn;
    // initialize trap_cx
    let mut trap_cx = TrapContext::app_init_context(
      entry_point,
      user_sp,
      KERNEL_SPACE.exclusive_access().token(),
      self.kernel_stack.get_top(),
      trap_handler as usize,
    );
    trap_cx.x[10] = args.len();
    trap_cx.x[11] = argv_base;
    *inner.get_trap_cx() = trap_cx;
    // **** release inner automatically
  }

  pub fn fork(
    self: &Arc<TaskControlBlock>,
  ) -> Arc<TaskControlBlock> {
    // ---- access parent PCB exclusively
    let mut parent_inner = self.inner_exclusive_access();
    // copy use space(include trap context)
    let memory_set = MemorySet::from_existed_user(
      &parent_inner.memory_set,
    );
    let trap_cx_ppn = memory_set
      .translate(VirtAddr::from(TRAP_CONTEXT).into())
      .unwrap()
      .ppn();
    // alloc a pid and a kernel stack in kernel space
    let pid_handle = pid_alloc();
    let kernel_stack = KernelStack::new(&pid_handle);
    let kernel_stack_top = kernel_stack.get_top();
    // copy fd table
    let mut new_fd_table: Vec<
      Option<Arc<dyn File + Send + Sync>>,
    > = Vec::new();
    for fd in parent_inner.fd_table.iter() {
      if let Some(file) = fd {
        new_fd_table.push(Some(file.clone()));
      } else {
        new_fd_table.push(None);
      }
    }
    let task_control_block = Arc::new(TaskControlBlock {
      pid: pid_handle,
      kernel_stack,
      inner: unsafe {
        UPSafeCell::new(TaskControlBlockInner {
          trap_cx_ppn,
          base_size: parent_inner.base_size,
          task_cx: TaskContext::goto_trap_return(
            kernel_stack_top,
          ),
          task_status: TaskStatus::Ready,
          memory_set,
          parent: Some(Arc::downgrade(self)),
          children: Vec::new(),
          exit_code: 0,
          fd_table: new_fd_table,
          signals: SignalFlags::empty(),
          // inherit the signal_mask and signal_action
          signal_mask: parent_inner.signal_mask,
          handling_sig: -1,
          signal_actions: parent_inner
            .signal_actions
            .clone(),
          killed: false,
          frozen: false,
          trap_ctx_backup: None,
          user_time: 0,
          kernel_time: 0,
        })
      },
    });
    // add child
    parent_inner.children.push(task_control_block.clone());
    // modify kernel_sp in trap_cx
    // **** access children PCB exclusively
    let trap_cx = task_control_block
      .inner_exclusive_access()
      .get_trap_cx();
    trap_cx.kernel_sp = kernel_stack_top;
    // return
    task_control_block
    // ---- release parent PCB automatically
    // **** release children PCB automatically
  }

  pub fn getpid(&self) -> usize {
    self.pid.0
  }
}

/// Task control block structure
pub struct TaskControlBlockInner {
  /// Indicates the physical page number of the physical page frame,
  /// in which the Trap context in the application address space is placed.
  pub trap_cx_ppn: PhysPageNum,
  /// Application data is only possible in areas,
  /// where the application address space is less than `base_size` bytes,
  /// With it, we can clearly know how much data the application has residing in memory.
  pub base_size: usize,

  /// Save the task context of the suspended task in the task control block.
  pub task_cx: TaskContext,
  /// Maintain the execution state of the current process.
  pub task_status: TaskStatus,
  /// Represents the application address space.
  pub memory_set: MemorySet,

  /// Points to the parent process of the current process.
  pub parent: Option<Weak<TaskControlBlock>>,
  /// Save the PCB of all child processes of the current process
  /// in a `Vec` in the form of `Arc` smart pointers, so that they can be found more easily.
  pub children: Vec<Arc<TaskControlBlock>>,
  /// when the process system call `exit()` or execution meets error and terminated by kernel,
  /// `exit_code` will save in its PCB, and then waiting for recycle its resources by it's parent process
  /// by calling `waitpid()`.
  pub exit_code: i32,

  /// Type description:
  /// - Vec: dynamic length.
  /// - Option: we can distinguish the file descriptor whether is
  ///   in free status(None) or being occupying(Some).
  /// - Arc: sharing reference.
  /// - dyn: maybe `Stdin` / `Stdout`
  pub fd_table: Vec<Option<Arc<dyn File + Send + Sync>>>,

  /// global signal set mask in current process.
  pub signal_mask: SignalFlags,
  /// A array with elements of `SignalAction` in fixed length.
  /// each of these records how the process responded to the corresponding signal.
  pub signal_actions: SignalActions,

  /// Record which signals have been received by the corresponding process
  /// and have not yet been processed.
  pub signals: SignalFlags,

  /// whether the process received `SIGSTOP` signal, to stop.
  pub killed: bool,
  /// whether the process is killed.
  pub frozen: bool,

  /// Processing routine for which signal the process is executing.
  pub handling_sig: isize,
  /// Trap context before the process executes the signal processing routine.
  pub trap_ctx_backup: Option<TrapContext>,

  pub user_time: usize,
  pub kernel_time: usize,
}

impl TaskControlBlockInner {
  pub fn get_trap_cx(&self) -> &'static mut TrapContext {
    self.trap_cx_ppn.get_mut()
  }

  pub fn get_user_token(&self) -> usize {
    self.memory_set.token()
  }

  fn get_status(&self) -> TaskStatus {
    self.task_status
  }

  pub fn is_zombie(&self) -> bool {
    self.get_status() == TaskStatus::Zombie
  }

  /// Allocate a minimum free file descriptor,
  /// otherwise extending the fd_table length and allocate one.
  pub fn alloc_fd(&mut self) -> usize {
    if let Some(fd) = (0..self.fd_table.len())
      .find(|fd| self.fd_table[*fd].is_none())
    {
      fd
    } else {
      self.fd_table.push(None);
      self.fd_table.len() - 1
    }
  }
}
