use riscv::register::sstatus::{self, Sstatus, SPP};

/// Trap Context
pub struct TrapContext {
  /// general regs[0..31]
  pub x: [usize; 32],
  /// CSR sstatus
  pub sstatus: Sstatus,
  /// CSR spec
  pub spec: usize,
}

impl TrapContext {
  /// set stack pointer to x_2 reg(sp)
  pub fn set_sp(&mut self, sp: usize) {
    self.x[2] = sp;
  }
  /// init app context
  pub fn app_init_context(entry: usize, sp: usize) -> Self {
    let mut sstatus = sstatus::read();// CSR sstatus
    sstatus.set_spp(SPP::User); // previous privilege mode: user mod
    let mut cx = Self {
      x: [0; 32],
      sstatus,
      spec: entry, // entry point of app
    };
    cx.set_sp(sp); // app's user stack pointer
    cx // return initial Trap Context of app
  }
}