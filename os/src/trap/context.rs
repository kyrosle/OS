use riscv::register::sstatus::{self, Sstatus, SPP};

#[repr(C)]
#[derive(Debug, Clone, Copy)]
/// Trap Context
pub struct TrapContext {
  /// general registers[0..31]
  pub x: [usize; 32],
  /// CSR sstatus
  pub sstatus: Sstatus,
  /// CSR spec
  pub sepc: usize,
  /// A token representing the kernel address space,
  /// that is, the starting physical address of the kernel page table.
  pub kernel_satp: usize,
  /// Represents the virtual address of the top of the kernel stack
  /// currently applied in the kernel address space
  pub kernel_sp: usize,
  /// The virtual address of the kernel trap handler entrypoint.
  pub trap_handler: usize,
}

impl TrapContext {
  /// set stack pointer to x2 register(sp)
  pub fn set_sp(&mut self, sp: usize) {
    self.x[2] = sp;
  }

  /// init app context
  pub fn app_init_context(
    entry: usize,
    sp: usize,
    kernel_satp: usize,
    kernel_sp: usize,
    trap_handler: usize,
  ) -> Self {
    let mut sstatus = sstatus::read(); // CSR sstatus
    sstatus.set_spp(SPP::User); // previous privilege mode: user mode
    let mut cx = Self {
      x: [0; 32],
      sstatus,
      sepc: entry,  // entry point of app
      kernel_satp,  // address of page table
      kernel_sp,    // kernel stack
      trap_handler, // address of trap_handler function
    };
    cx.set_sp(sp); // app's user stack pointer
    cx // return initial Trap Context of app
  }
}
