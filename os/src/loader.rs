//! Loading user application into memory
//!
//! For chapter 3, user application are simply part of the data included in the
//! kernel binary, so we only need to copy them to the space allocated for each
//! app to load them. We also allocate fixed spaces for each task's
//! [`KernelStack`] and [`UserStack`].

use crate::config::*;
use core::arch::asm;
use lazy_static::lazy_static;

use crate::{println, sync::UPSafeCell, trap::context::TrapContext};

lazy_static! {
  static ref APP_MANAGER: UPSafeCell<AppManager> = unsafe {
    UPSafeCell::new({
      extern "C" {
        fn _num_app();
      }
      let num_app_ptr = _num_app as usize as *const usize;
      let num_app = num_app_ptr.read_volatile();
      let mut app_start: [usize; MAX_APP_NUM + 1] = [0; MAX_APP_NUM + 1];
      let app_start_raw: &[usize] =
        core::slice::from_raw_parts(num_app_ptr.add(1), num_app + 1);

      app_start[..=num_app].copy_from_slice(app_start_raw);

      AppManager {
        num_app,
        current_app: 0,
        app_start,
      }
    })
  };
}

// 8 KiB
static KERNEL_STACK: [KernelStack; MAX_APP_NUM] = [KernelStack {
  data: [0; KERNEL_STACK_SIZE],
}; MAX_APP_NUM];
static USER_STACK: [UserStack; MAX_APP_NUM] = [UserStack {
  data: [0; USER_STACK_SIZE],
}; MAX_APP_NUM];

#[repr(align(4096))]
#[derive(Copy, Clone)]
struct KernelStack {
  data: [u8; KERNEL_STACK_SIZE],
}

impl KernelStack {
  fn get_sp(&self) -> usize {
    self.data.as_ptr() as usize + KERNEL_STACK_SIZE
  }
  pub fn push_context(&self, trap_cx: TrapContext) -> usize {
    let trap_cx_ptr =
      (self.get_sp() - core::mem::size_of::<TrapContext>()) as *mut TrapContext;
    // move the TrapContext
    // position:
    // High Address
    // ( 4096 bytes )
    // +------------+ <- Kernel stack sp
    // |////////////|
    // |Trap Context|
    // |////////////| (sp - core::mem::size_of::<TrapContext>())
    // |            |
    // |            |
    // +------------+
    // Low Address
    unsafe {
      *trap_cx_ptr = trap_cx;
    }

    trap_cx_ptr as usize
  }
}

#[repr(align(4096))]
#[derive(Copy, Clone)]
struct UserStack {
  data: [u8; USER_STACK_SIZE],
}

impl UserStack {
  fn get_sp(&self) -> usize {
    self.data.as_ptr() as usize + USER_STACK_SIZE
  }
}
struct AppManager {
  num_app: usize,
  current_app: usize,
  app_start: [usize; MAX_APP_NUM + 1],
}

impl AppManager {
  pub fn print_app_info(&self) {
    println!("[kernel] num_app = {}", self.num_app);
    for i in 0..self.num_app {
      println!(
        "[kernel] app_{} [{:#x}, {:#x}]",
        i,
        self.app_start[i],
        self.app_start[i + 1],
      );
    }
  }

  /// load the `app_id` application into the 0x80400000 address,
  ///
  /// It essentially copies data from one piece of memory to another.
  unsafe fn load_app(&self, app_id: usize) {
    if app_id >= self.num_app {
      panic!("All applications completed!");
    }
    println!("[kernel] Loading app_{}", app_id);

    // clear app area

    // clear the application area and fill it with zero.
    core::slice::from_raw_parts_mut(
      APP_BASE_ADDRESS as *mut u8,
      APP_SIZE_LIMIT,
    )
    .fill(0);
    // form the app code data slice.
    let app_src = core::slice::from_raw_parts(
      self.app_start[app_id] as *const u8,
      self.app_start[app_id + 1] - self.app_start[app_id],
    );

    // copy the slice code to the destination area(0x80400000).
    let app_dst = core::slice::from_raw_parts_mut(
      APP_BASE_ADDRESS as *mut u8,
      app_src.len(),
    );
    app_dst.copy_from_slice(app_src);

    // memory fence about fetching the instruction memory
    // The fetch process after it must be able to see all previous
    // modifications to the fetch memory area
    asm!("fence.i");
  }

  pub fn get_current_app(&self) -> usize {
    self.current_app
  }

  pub fn move_to_next_app(&mut self) {
    self.current_app += 1;
  }
}

/// init batch subsystem
pub fn init() {
  print_app_info();
}

/// print app info
pub fn print_app_info() {
  APP_MANAGER.exclusive_access().print_app_info();
}

/// Load n-th user app at
/// [APP_BASE_ADDRESS + n * APP_SIZE_LIMIT, APP_BASE_ADDRESS + (n + 1) * APP_SIZE_LIMIT).
pub fn load_apps() {
  extern "C" {
    fn _num_app();
  }
  let num_app_ptr = _num_app as usize as *const usize;
  let num_app = get_num_app();
  let app_start =
    unsafe { core::slice::from_raw_parts(num_app_ptr.add(1), num_app + 1) };
  // clear i-cache first
  unsafe {
    // use the `volatile` keyword to prevent the compiler from rearranging instructions
    // asm!("fence.i" :::: "volatile");
    asm!("fence.i");
  }
  // load apps
  for i in 0..num_app {
    let base_i = get_base_i(i);
    // clear region
    (base_i..base_i + APP_SIZE_LIMIT)
      .for_each(|addr| unsafe { (addr as *mut u8).write_volatile(0) });
    // load app from data section to memory
    let src = unsafe {
      core::slice::from_raw_parts(
        app_start[i] as *const u8,
        app_start[i + 1] - app_start[i],
      )
    };
    let dst =
      unsafe { core::slice::from_raw_parts_mut(base_i as *mut u8, src.len()) };

    dst.copy_from_slice(src);
  }
}

/// Get base address of i-th app.
fn get_base_i(app_id: usize) -> usize {
  APP_BASE_ADDRESS + app_id * APP_SIZE_LIMIT
}

/// Get the total number of applications.
pub fn get_num_app() -> usize {
  extern "C" {
    fn _num_app();
  }
  unsafe { (_num_app as usize as *const usize).read_volatile() }
}

/// get app info with entry and sp and save `TrapContext` in kernel stack.
pub fn init_app_cx(app_id: usize) -> usize {
  KERNEL_STACK[app_id].push_context(TrapContext::app_init_context(
    get_base_i(app_id),
    USER_STACK[app_id].get_sp(),
  ))
}

///// run next app
// pub fn run_next_app() -> ! {
//   let mut app_manager = APP_MANAGER.exclusive_access();
//   let current_app = app_manager.get_current_app();
//   unsafe {
//     app_manager.load_app(current_app);
//   }
//   app_manager.move_to_next_app();
//   drop(app_manager);

//   // before this we have to drop local variables related to resources manually
//   // and release the resources
//   extern "C" {
//     fn __restore(cx_addr: usize);
//   }

//   unsafe {
//     __restore(KERNEL_STACK.push_context(TrapContext::app_init_context(
//       APP_BASE_ADDRESS,
//       USER_STACK.get_sp(),
//     )) as *const _ as usize);
//   }

//   panic!("Unreachable in batch::run_current_app!");
// }
