use alloc::{
  sync::{Arc, Weak},
  vec::Vec,
};
use lazy_static::*;

use crate::{
  config::{
    KERNEL_STACK_SIZE, PAGE_SIZE, TRAMPOLINE,
    TRAP_CONTEXT_BASE, USER_STACK_SIZE,
  },
  mm::{
    MapPermission, PhysPageNum, VirtAddr, KERNEL_SPACE,
  },
  sync::UPSafeCell,
};

use super::ProcessControlBlock;

lazy_static! {
  pub static ref PID_ALLOCATOR: UPSafeCell<RecycleAllocator> =
    unsafe { UPSafeCell::new(RecycleAllocator::new()) };
  pub static ref KSTACK_ALLOCATOR: UPSafeCell<RecycleAllocator> =
    unsafe { UPSafeCell::new(RecycleAllocator::new()) };
}

/// Allocate a pid from PID_ALLOCATOR
pub fn pid_alloc() -> PidHandle {
  PidHandle(PID_ALLOCATOR.exclusive_access().alloc())
}

/// Bind pid lifetime to `PidHandle`
pub struct PidHandle(pub usize);

impl Drop for PidHandle {
  fn drop(&mut self) {
    println!("drop pid {}", self.0);
    PID_ALLOCATOR.exclusive_access().dealloc(self.0);
  }
}

/// Universal Allocator structure
pub struct RecycleAllocator {
  current: usize,
  recycled: Vec<usize>,
}

impl RecycleAllocator {
  /// Create an empty `PidAllocator`
  pub fn new() -> Self {
    RecycleAllocator {
      current: 0,
      recycled: Vec::new(),
    }
  }

  /// Allocate a id
  pub fn alloc(&mut self) -> usize {
    if let Some(id) = self.recycled.pop() {
      id
    } else {
      self.current += 1;
      self.current - 1
    }
  }

  /// Recycle a id
  pub fn dealloc(&mut self, id: usize) {
    assert!(id < self.current);
    assert!(
      !self.recycled.iter().any(|i| *i == id),
      "id {} has been deallocated!",
      id
    );
    self.recycled.push(id);
  }
}

pub struct KernelStack(pub usize);

/// Return (bottom, top) of a kernel stack in kernel space.
pub fn kernel_stack_position(
  app_id: usize,
) -> (usize, usize) {
  let top =
    TRAMPOLINE - app_id * (KERNEL_STACK_SIZE + PAGE_SIZE);
  let bottom = top - KERNEL_STACK_SIZE;
  (bottom, top)
}

/// Allocate a Kernel Stack
pub fn kstack_alloc() -> KernelStack {
  let kstack_id =
    KSTACK_ALLOCATOR.exclusive_access().alloc();
  let (kstack_bottom, kstack_top) =
    kernel_stack_position(kstack_id);
  KERNEL_SPACE.exclusive_access().insert_framed_area(
    kstack_bottom.into(),
    kstack_top.into(),
    MapPermission::R | MapPermission::W,
  );
  KernelStack(kstack_id)
}

impl KernelStack {
  #[allow(unused)]
  /// Push a value on top of kernel stack
  pub fn push_on_top<T>(&self, value: T) -> *mut T
  where
    T: Sized,
  {
    let kernel_stack_top = self.get_top();
    let ptr_mut = (kernel_stack_top
      - core::mem::size_of::<T>())
      as *mut T;
    unsafe {
      *ptr_mut = value;
    }
    ptr_mut
  }

  /// Get the value on the top of kernel stack
  pub fn get_top(&self) -> usize {
    let (_, kernel_stack_top) =
      kernel_stack_position(self.0);
    kernel_stack_top
  }
}

impl Drop for KernelStack {
  fn drop(&mut self) {
    let (kernel_stack_bottom, _) =
      kernel_stack_position(self.0);
    let kernel_stack_bottom_va: VirtAddr =
      kernel_stack_bottom.into();
    KERNEL_SPACE
      .exclusive_access()
      .remove_area_with_start_vpn(
        kernel_stack_bottom_va.into(),
      );
    KSTACK_ALLOCATOR.exclusive_access().dealloc(self.0)
  }
}

/// Thread Resource Set
pub struct TaskUserRes {
  /// TID allocated by current process.
  pub tid: usize,
  /// Used to calculate the thread user stack position.
  pub ustack_base: usize,
  /// Weak reference to the current process.
  pub process: Weak<ProcessControlBlock>,
}

/// Acquire the address of the corresponding TrapContext.
fn trap_cx_bottom_from_tid(tid: usize) -> usize {
  TRAP_CONTEXT_BASE - tid * PAGE_SIZE
}

/// Acquire the address of the corresponding UserStack.
fn ustack_bottom_from_tid(
  ustack_base: usize,
  tid: usize,
) -> usize {
  ustack_base + tid * (PAGE_SIZE + USER_STACK_SIZE)
}

impl TaskUserRes {
  /// Create a new thread,
  /// `alloc_user_res` controlling whether mapping a new UserStack and TrapContext.
  /// Such as, in the operation of `fork`, child process fork a new process, we don't
  /// have to allocate a new UserStack and TrapContext, because the child process has
  /// copied its father address space, here, the `alloc_user_res` is false.
  pub fn new(
    process: Arc<ProcessControlBlock>,
    ustack_base: usize,
    alloc_user_res: bool,
  ) -> Self {
    let tid = process.inner_exclusive_access().alloc_tid();
    let task_user_res = Self {
      tid,
      ustack_base,
      process: Arc::downgrade(&process),
    };

    if alloc_user_res {
      task_user_res.alloc_user_res();
    }
    task_user_res
  }

  /// Mapping the UserStack and TrapContext of thread in current process space.
  pub fn alloc_user_res(&self) {
    let process = self.process.upgrade().unwrap();
    let mut process_inner =
      process.inner_exclusive_access();

    // alloc user stack
    let ustack_bottom =
      ustack_bottom_from_tid(self.ustack_base, self.tid);
    let ustack_top = ustack_bottom + USER_STACK_SIZE;
    process_inner.memory_set.insert_framed_area(
      ustack_bottom.into(),
      ustack_top.into(),
      MapPermission::R
        | MapPermission::W
        | MapPermission::U,
    );

    // alloc trap_cx
    let trap_cx_bottom = trap_cx_bottom_from_tid(self.tid);
    let trap_cx_top = trap_cx_bottom + PAGE_SIZE;
    process_inner.memory_set.insert_framed_area(
      trap_cx_bottom.into(),
      trap_cx_top.into(),
      MapPermission::R | MapPermission::W,
    )
  }

  /// Deallocate Thread resources, including user stack, TrapContext UMapping.
  fn dealloc_user_res(&self) {
    // dealloc tid
    let process = self.process.upgrade().unwrap();
    let mut process_inner =
      process.inner_exclusive_access();
    // dealloc ustack manually
    let ustack_bottom_va: VirtAddr =
      ustack_bottom_from_tid(self.ustack_base, self.tid)
        .into();
    process_inner
      .memory_set
      .remove_area_with_start_vpn(ustack_bottom_va.into());

    // dealloc trap_cx manually
    let trap_cx_bottom_va: VirtAddr =
      trap_cx_bottom_from_tid(self.tid).into();
    process_inner
      .memory_set
      .remove_area_with_start_vpn(trap_cx_bottom_va.into());
  }

  /// Deallocate thread identifier.
  pub fn dealloc_tid(&self) {
    let process = self.process.upgrade().unwrap();
    let mut process_inner =
      process.inner_exclusive_access();
    process_inner.dealloc_tid(self.tid);
  }

  /// Acquire the virtual address of TrapContext.
  pub fn trap_cx_user_va(&self) -> usize {
    trap_cx_bottom_from_tid(self.tid)
  }

  /// Acquire the Physical Page Number of TrapContext.
  pub fn trap_cx_ppn(&self) -> PhysPageNum {
    let process = self.process.upgrade().unwrap();
    let process_inner = process.inner_exclusive_access();
    let trap_cx_bottom_va: VirtAddr =
      trap_cx_bottom_from_tid(self.tid).into();
    // translate the virtual address into physical page number.
    process_inner
      .memory_set
      .translate(trap_cx_bottom_va.into())
      .unwrap()
      .ppn()
  }

  /// Acquire the user stack base address.
  pub fn ustack_base(&self) -> usize {
    self.ustack_base
  }

  /// Acquire the user stack top address.
  pub fn ustack_top(&self) -> usize {
    ustack_bottom_from_tid(self.ustack_base, self.tid)
      + USER_STACK_SIZE
  }
}

/// RALL
impl Drop for TaskUserRes {
  fn drop(&mut self) {
    self.dealloc_tid();
    self.dealloc_user_res();
  }
}
