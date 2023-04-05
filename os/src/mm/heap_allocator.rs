use crate::config::KERNEL_HEAP_SIZE;
use buddy_system_allocator::LockedHeap;

#[global_allocator]
/// heap allocator instance
static HEAP_ALLOCATOR: LockedHeap<32> = LockedHeap::empty();

/// heap space ([u8; KERNEL_HEAP_SIZE])
static mut HEAP_SPACE: [u8; KERNEL_HEAP_SIZE] = [0; KERNEL_HEAP_SIZE];

/// initializes heap allocator
pub fn init_heap() {
  unsafe {
    HEAP_ALLOCATOR
      .lock()
      .init(HEAP_SPACE.as_ptr() as usize, KERNEL_HEAP_SIZE)
  }
}

#[alloc_error_handler]
pub fn handle_alloc_error(layout: core::alloc::Layout) -> ! {
  panic!("Heap allocation error, layout = {:?}", layout);
}

#[allow(unused)]
pub fn heap_test() {
  use alloc::boxed::Box;
  use alloc::vec::Vec;
  extern "C" {
    fn sbss();
    fn ebss();
  }
  let bss_range = sbss as usize..ebss as usize;
  let a = Box::new(5);
  assert_eq!(*a, 5);
  assert!(bss_range.contains(&(a.as_ref() as *const _ as usize)));
  drop(a);
  let mut v: Vec<usize> = Vec::new();
  for i in 0..500 {
    v.push(i);
  }
  (0..500).into_iter().for_each(|i| assert_eq!(v[i], i));
  // for i in 0..500 {
  //   assert_eq!(v[i], i);
  // }
  drop(v);
  println!("heap_test passed!");
}