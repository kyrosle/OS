use alloc::sync::Arc;

use crate::{block_cache::get_block_cache, block_dev::BlockDevice, BLOCK_SZ};

/// Decompose bits into (block_pos, bits64_pos, inner_pos)
fn decomposition(mut bit: usize) -> (usize, usize, usize) {
  let block_pos = bit / BLOCK_BITS;
  bit %= BLOCK_BITS;
  (block_pos, bit / 64, bit % 64)
}

type BitmapBlock = [u64; 64];
const BLOCK_BITS: usize = BLOCK_SZ * 8;

pub struct Bitmap {
  start_block_id: usize,
  blocks: usize,
}

impl Bitmap {
  pub fn new(start_block_id: usize, blocks: usize) -> Self {
    Self {
      start_block_id,
      blocks,
    }
  }
  pub fn alloc(&self, block_device: &Arc<dyn BlockDevice>) -> Option<usize> {
    // enumerate each block(block_id), we are trying to find a free bit within a block and set it as 1.
    for block_id in 0..self.blocks {
      // we use `get_block_cache` to get the block-cache,
      // mention: the `block_id` we pass in is the `start_block_id + block_id` to get the correspond block id.
      let pos = get_block_cache(
        block_id + self.start_block_id,
        Arc::clone(block_device),
      )
      .lock()
      .modify(0, |bitmap_block: &mut BitmapBlock| {
        // offset is 0, because the whole block only has a `BitmapBlock` (512 bytes).
        // Iterate through the groups consisting of 64 bits, if this group (u64) has not reached to the end.
        // using `u64::trailing_ones` to find the lowest bit of `0` and set it as `1`,
        // then save this bit group number into `bits64_pos`, the location of allocated bit will store at `inner_pos`
        // Calculating the location of allocated bit: block_id * BLOCK_BITS + bits64_pos * 64 + inner_pos .
        // The `block_id` variable from outside.
        if let Some((bits64_pos, inner_pos)) = bitmap_block
          .iter()
          .enumerate()
          .find(|(_, bits64)| **bits64 != u64::MAX)
          .map(|(bits64_pos, bits64)| {
            (bits64_pos, bits64.trailing_ones() as usize)
          })
        {
          // modify cache
          bitmap_block[bits64_pos] |= 1u64 << inner_pos;
          Some(block_id * BLOCK_BITS + bits64_pos * 64 + inner_pos)
        } else {
          // doesn't exist
          None
        }
      });
      if pos.is_some() {
        return pos;
      }
    }
    None
  }

  /// Deallocate a block
  pub fn dealloc(&self, block_device: &Arc<dyn BlockDevice>, bit: usize) {
    let (block_pos, bits64_pos, inner_pos) = decomposition(bit);
    get_block_cache(block_pos + self.start_block_id, Arc::clone(block_device))
      .lock()
      .modify(0, |bitmap_block: &mut BitmapBlock| {
        assert!(bitmap_block[bits64_pos] & (1u64 << inner_pos) > 0);
        bitmap_block[bits64_pos] -= 1u64 << inner_pos;
      });
  }

  /// Get the max number of allocatable blocks
  pub fn maximum(&self) -> usize {
    self.blocks * BLOCK_BITS
  }
}
