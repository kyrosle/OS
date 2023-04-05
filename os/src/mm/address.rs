use crate::config::PAGE_SIZE_BITS;

#[derive(Copy, Clone, Ord, PartialEq, Eq, PartialOrd)]
pub struct PhysAddr(pub usize);

#[derive(Copy, Clone, Ord, PartialEq, Eq, PartialOrd)]
pub struct VirtAddr(pub usize);

#[derive(Copy, Clone, Ord, PartialEq, Eq, PartialOrd)]
pub struct PhysPageNum(pub usize);

#[derive(Copy, Clone, Ord, PartialEq, Eq, PartialOrd)]
pub struct VirtPageNum(pub usize);

// physical address

// 38                         12 11            0
//  +---------------------------+--------------+
//  |    virtual page number    |  page offset |
//  +---------------------------+--------------+
//              VPN                  12-bit

// 55                                           12 11           0
//  +---------------------------------------------+-------------+
//  |              physical page number           | page offset |
//  +---------------------------------------------+-------------+
//                          PPN                        12-bit

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
