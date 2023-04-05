use crate::config::{PAGE_SIZE, PAGE_SIZE_BITS};

#[derive(Copy, Clone, Ord, PartialEq, Eq, PartialOrd)]
pub struct PhysAddr(pub usize);

#[derive(Copy, Clone, Ord, PartialEq, Eq, PartialOrd)]
pub struct VirtAddr(pub usize);

#[derive(Copy, Clone, Ord, PartialEq, Eq, PartialOrd)]
pub struct PhysPageNum(pub usize);

#[derive(Copy, Clone, Ord, PartialEq, Eq, PartialOrd)]
pub struct VirtPageNum(pub usize);

// Virtual Address
// 38                         12 11            0
//  +---------------------------+--------------+
//  |    virtual page number    |  page offset |
//  +---------------------------+--------------+
//              VPN                  12-bit

// Physical Address
// 55                                           12 11           0
//  +---------------------------------------------+-------------+
//  |              physical page number           | page offset |
//  +---------------------------------------------+-------------+
//                          PPN                        12-bit

// Page Table Entry, PTE
// 63      54 53    28 27    19 18    10 9   8 7 6 5 4 3 2 1 0
//  +--------+--------+--------+--------+-----+-+-+-+-+-+-+-+-+
//  |Reserved| PPN[2] | PPN[1] | PPN[0] | RSW |D|A|G|U|X|W|R|V|
//  +--------+--------+--------+--------+-----+-+-+-+-+-+-+-+-+
//     10        26       9         9      2   1 1 1 1 1 1 1 1
//                                             | |   | | | | |-> validated
//                                             D Acc U | | |-> Read
//                                             i ess S | |-> Write
//                                             r ed  E |-> Execute
//                                             t     R
//                                             y
const PA_WIDTH_SV39: usize = 56;
const VA_WIDTH_SV39: usize = 39;
const PPN_WIDTH_SV39: usize = PA_WIDTH_SV39 - PAGE_SIZE_BITS;
const VPN_WIDTH_SV39: usize = VA_WIDTH_SV39 - PAGE_SIZE_BITS;

impl From<usize> for PhysAddr {
  fn from(v: usize) -> Self {
    Self(v & ((1 << PA_WIDTH_SV39) - 1))
  }
}

impl From<usize> for PhysPageNum {
  fn from(v: usize) -> Self {
    Self(v & ((1 << PPN_WIDTH_SV39) - 1))
  }
}

impl From<PhysAddr> for usize {
  fn from(v: PhysAddr) -> Self {
    v.0
  }
}

impl From<PhysPageNum> for usize {
  fn from(v: PhysPageNum) -> Self {
    v.0
  }
}

impl PhysAddr {
  pub fn page_offset(&self) -> usize {
    self.0 & (PAGE_SIZE - 1)
  }
  pub fn floor(&self) -> PhysPageNum {
    PhysPageNum(self.0 / PAGE_SIZE)
  }
  pub fn ceil(&self) -> PhysPageNum {
    PhysPageNum((self.0 + PAGE_SIZE - 1) / PAGE_SIZE)
  }
}

impl From<PhysAddr> for PhysPageNum {
  fn from(v: PhysAddr) -> Self {
    assert_eq!(v.page_offset(), 0);
    v.floor()
  }
}

impl From<PhysPageNum> for PhysAddr {
  fn from(v: PhysPageNum) -> Self {
    PhysAddr(v.0 << PAGE_SIZE_BITS)
  }
}
