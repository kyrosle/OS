use crate::{
  mm::{
    frame_alloc, frame_dealloc, kernel_token, FrameTracker,
    PageTable, PhysAddr, PhysPageNum, StepByOne, VirtAddr,
  },
  sync::UPSafeCell,
};
use alloc::vec::Vec;
use easyfs::BlockDevice;
use lazy_static::lazy_static;
use virtio_drivers::{Hal, VirtIOBlk, VirtIOHeader};

const VIRTIO0: usize = 0x1000_1000;

lazy_static! {
  static ref QUEUE_FRAMES: UPSafeCell<Vec<FrameTracker>> =
    unsafe { UPSafeCell::new(Vec::new()) };
}

pub struct VirtIOBlock(
  UPSafeCell<VirtIOBlk<'static, VirtioHal>>,
);

impl BlockDevice for VirtIOBlock {
  fn read_block(&self, block_id: usize, buf: &mut [u8]) {
    self
      .0
      .exclusive_access()
      .read_block(block_id, buf)
      .expect("Error when reading VirtIOBlk")
  }

  fn write_block(&self, block_id: usize, buf: &[u8]) {
    self
      .0
      .exclusive_access()
      .write_block(block_id, buf)
      .expect("Error when writing VirtIOBlk")
  }
}

impl VirtIOBlock {
  #[allow(unused)]
  pub fn new() -> Self {
    unsafe {
      Self(UPSafeCell::new(
        VirtIOBlk::<VirtioHal>::new(
          &mut *(VIRTIO0 as *mut VirtIOHeader),
        )
        .unwrap(),
      ))
    }
  }
}

pub struct VirtioHal;

impl Hal for VirtioHal {
  fn dma_alloc(pages: usize) -> virtio_drivers::PhysAddr {
    let mut ppn_base = PhysPageNum(0);
    for i in 0..pages {
      let frame = frame_alloc().unwrap();
      if i == 0 {
        ppn_base = frame.ppn;
      }
      assert_eq!(frame.ppn.0, ppn_base.0 + i);
      QUEUE_FRAMES.exclusive_access().push(frame);
    }
    let pa: PhysAddr = ppn_base.into();
    pa.0
  }

  fn dma_dealloc(
    pa: virtio_drivers::PhysAddr,
    pages: usize,
  ) -> i32 {
    let pa = PhysAddr::from(pa);
    let mut ppn_base: PhysPageNum = pa.into();
    for _ in 0..pages {
      frame_dealloc(ppn_base);
      ppn_base.step();
    }
    0
  }

  fn phys_to_virt(
    paddr: virtio_drivers::PhysAddr,
  ) -> virtio_drivers::VirtAddr {
    paddr
  }

  fn virt_to_phys(
    vaddr: virtio_drivers::VirtAddr,
  ) -> virtio_drivers::PhysAddr {
    PageTable::from_token(kernel_token())
      .translate_va(VirtAddr::from(vaddr))
      .unwrap()
      .0
  }
}
