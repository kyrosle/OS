use core::cell::RefMut;

use alloc::string::String;
use alloc::vec;
use alloc::{
  sync::{Arc, Weak},
  vec::Vec,
};

use crate::fs::{Stdin, Stdout};
use crate::mm::{translated_refmut, KERNEL_SPACE};
use crate::sync::{Condvar, Mutex, Semaphore};
use crate::trap::{trap_handler, TrapContext};
use crate::{fs::File, mm::MemorySet, sync::UPSafeCell};

use super::{
  add_task, insert_into_pid2process, pid_alloc, PidHandle,
  RecycleAllocator, SignalFlags, TaskControlBlock,
};

/// Process Control Block
pub struct ProcessControlBlock {
  /// immutable
  /// Process Identifier.
  pub pid: PidHandle,
  /// mutable
  inner: UPSafeCell<ProcessControlBlockInner>,
}

impl ProcessControlBlock {
  pub fn inner_exclusive_access(
    &self,
  ) -> RefMut<'_, ProcessControlBlockInner> {
    self.inner.exclusive_access()
  }

  pub fn new(elf_data: &[u8]) -> Arc<Self> {
    // memory_set with elf program headers/trampoline/trap-context/user-stack
    let (memory_set, ustack_base, entry_point) =
      MemorySet::from_elf(elf_data);

    // alloc a pid
    let pid_handle = pid_alloc();

    // create PCB
    let process = Arc::new(Self {
      pid: pid_handle,
      inner: unsafe {
        UPSafeCell::new(ProcessControlBlockInner {
          is_zombie: false,
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
          tasks: Vec::new(),
          task_res_allocator: RecycleAllocator::new(),
          mutex_list: Vec::new(),
          semaphore_list: Vec::new(),
          condvar_list: Vec::new(),
        })
      },
    });

    // create a main thread, we should allocate ustack and trap_cx here
    let task = Arc::new(TaskControlBlock::new(
      Arc::clone(&process),
      ustack_base,
      true,
    ));

    // prepare trap_cx of main thread
    let task_inner = task.inner_exclusive_access();
    let trap_cx = task_inner.get_trap_cx();
    let ustack_top =
      task_inner.res.as_ref().unwrap().ustack_top();
    let kstack_top = task.kstack.get_top();
    drop(task_inner);
    *trap_cx = TrapContext::app_init_context(
      entry_point,
      ustack_top,
      KERNEL_SPACE.exclusive_access().token(),
      kstack_top,
      trap_handler as usize,
    );
    // add main thread to the process
    let mut process_inner =
      process.inner_exclusive_access();
    process_inner.tasks.push(Some(Arc::clone(&task)));
    drop(process_inner);
    insert_into_pid2process(
      process.getpid(),
      Arc::clone(&process),
    );
    // add main thread to scheduler
    add_task(task);
    process
  }

  /// Only support processes with a single thread.
  pub fn exec(
    self: &Arc<Self>,
    elf_data: &[u8],
    args: Vec<String>,
  ) {
    assert_eq!(
      self.inner_exclusive_access().thread_count(),
      1
    );
    // memory_set with elf program headers/trampoline/trap context/user stack
    let (memory_set, ustack_base, entry_point) =
      MemorySet::from_elf(elf_data);
    let new_token = memory_set.token();

    // substitute memory_set
    self.inner_exclusive_access().memory_set = memory_set;

    // then we alloc user resource for main thread again
    // since memory_set has been changed.
    let task = self.inner_exclusive_access().get_task(0);
    let mut task_inner = task.inner_exclusive_access();
    task_inner.res.as_mut().unwrap().ustack_base =
      ustack_base;
    task_inner.res.as_mut().unwrap().alloc_user_res();
    task_inner.trap_cx_ppn =
      task_inner.res.as_mut().unwrap().trap_cx_ppn();

    // push arguments to user stack
    let mut user_sp =
      task_inner.res.as_mut().unwrap().ustack_top();
    user_sp -=
      (args.len() + 1) * core::mem::size_of::<usize>();
    let argv_base = user_sp;
    let mut argv = (0..=args.len())
      .map(|arg| {
        translated_refmut(
          new_token,
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
        *translated_refmut(new_token, p as *mut u8) = *c;
        p += 1;
      }
      *translated_refmut(new_token, p as *mut u8) = 0;
    }
    // make user_sp aligned to 8B for k210 platform.
    user_sp -= user_sp % core::mem::size_of::<usize>();

    // initialize trap_cx
    let mut trap_cx = TrapContext::app_init_context(
      entry_point,
      user_sp,
      KERNEL_SPACE.exclusive_access().token(),
      task.kstack.get_top(),
      trap_handler as usize,
    );
    trap_cx.x[10] = args.len();
    trap_cx.x[11] = argv_base;
    *task_inner.get_trap_cx() = trap_cx;
  }

  /// Only support processes with a single thread.
  pub fn fork(self: &Arc<Self>) -> Arc<Self> {
    let mut parent = self.inner_exclusive_access();
    assert_eq!(parent.thread_count(), 1);
    // clone parent's memory_set completely including trampoline/ustacks/trap_cxs
    let memory_set =
      MemorySet::from_existed_user(&parent.memory_set);

    // alloc a pid
    let pid = pid_alloc();

    // copy fd table
    let mut new_fd_table: Vec<
      Option<Arc<dyn File + Send + Sync>>,
    > = Vec::new();
    for fd in parent.fd_table.iter() {
      if let Some(file) = fd {
        new_fd_table.push(Some(file.clone()));
      } else {
        new_fd_table.push(None);
      }
    }
    // create child process pcb
    let child = Arc::new(Self {
      pid,
      inner: unsafe {
        UPSafeCell::new(ProcessControlBlockInner {
          is_zombie: false,
          memory_set,
          children: Vec::new(),
          parent: Some(Arc::downgrade(self)),
          exit_code: 0,
          fd_table: new_fd_table,
          signals: SignalFlags::empty(),
          tasks: Vec::new(),
          task_res_allocator: RecycleAllocator::new(),
          mutex_list: Vec::new(),
          semaphore_list: Vec::new(),
          condvar_list: Vec::new(),
        })
      },
    });
    // add child
    parent.children.push(Arc::clone(&child));
    // create main thread of child process
    let task = Arc::new(TaskControlBlock::new(
      Arc::clone(&child),
      parent
        .get_task(0)
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .ustack_base(),
      // here we do not allocate trap_cx or ustack again
      // but mention that we allocate a new kstack here
      false,
    ));
    // attach task to child process
    let mut child_inner = child.inner_exclusive_access();
    child_inner.tasks.push(Some(Arc::clone(&task)));
    drop(child_inner);
    // modify kstack_top in trap_cx of this thread
    let task_inner = task.inner_exclusive_access();
    let trap_cx = task_inner.get_trap_cx();
    trap_cx.kernel_sp = task.kstack.get_top();
    drop(task_inner);
    insert_into_pid2process(
      child.getpid(),
      Arc::clone(&child),
    );
    // add this thread to scheduler
    add_task(task);
    child
  }

  pub fn getpid(&self) -> usize {
    self.pid.0
  }
}

pub struct ProcessControlBlockInner {
  /// whether current process is being killed.
  pub is_zombie: bool,

  /// Represents the application address space.
  pub memory_set: MemorySet,

  /// Points to the parent process of the current process.
  pub parent: Option<Weak<ProcessControlBlock>>,
  /// Save the PCB of all child processes of the current process
  /// in a `Vec` in the form of `Arc` smart pointers, so that they can be found more easily.
  pub children: Vec<Arc<ProcessControlBlock>>,
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

  /// Record which signals have been received by the corresponding process
  /// and have not yet been processed.
  pub signals: SignalFlags,

  /// Recording the threads generated by current process.
  pub tasks: Vec<Option<Arc<TaskControlBlock>>>,
  /// Used to allocate thread identifier.
  pub task_res_allocator: RecycleAllocator,

  pub mutex_list: Vec<Option<Arc<dyn Mutex>>>,
  pub semaphore_list: Vec<Option<Arc<Semaphore>>>,
  pub condvar_list: Vec<Option<Arc<Condvar>>>,
}

impl ProcessControlBlockInner {
  #[allow(unused)]
  pub fn get_user_token(&self) -> usize {
    self.memory_set.token()
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

  /// Allocate a minium free thread identifier.
  pub fn alloc_tid(&mut self) -> usize {
    self.task_res_allocator.alloc()
  }

  /// Deallocate the specified thread identifier.
  pub fn dealloc_tid(&mut self, tid: usize) {
    self.task_res_allocator.dealloc(tid)
  }

  /// Return the amount of threads in current process.
  pub fn thread_count(&self) -> usize {
    self.tasks.len()
  }

  /// Acquire the thread with the index of tid.
  pub fn get_task(
    &self,
    tid: usize,
  ) -> Arc<TaskControlBlock> {
    self.tasks[tid].as_ref().unwrap().clone()
  }
}
