use alloc::{collections::VecDeque, sync::Arc};
use lazy_static::lazy_static;
use spin::Mutex;

use crate::{block_dev::BlockDevice, BLOCK_SZ};

lazy_static! {
  pub static ref BLOCK_CACHE_MANAGER: Mutex<BlockCacheManager> =
    Mutex::new(BlockCacheManager::new());
}

/// Get the block cache corresponding to the given block id and block device
pub fn get_block_cache(
  block_id: usize,
  block_device: Arc<dyn BlockDevice>,
) -> Arc<Mutex<BlockCache>> {
  BLOCK_CACHE_MANAGER
    .lock()
    .get_block_cache(block_id, block_device)
}

/// Sync all block cache to block device
pub fn block_cache_sync_all() {
  let manager = BLOCK_CACHE_MANAGER.lock();
  for (_, cache) in manager.queue.iter() {
    cache.lock().sync();
  }
}

/// Cached block inside memory
pub struct BlockCache {
  /// cached block data
  cache: [u8; BLOCK_SZ],
  /// underlying block id
  block_id: usize,
  /// underlying block device
  block_device: Arc<dyn BlockDevice>,
  /// whether the block is dirty
  modified: bool,
}

impl BlockCache {
  /// Load a new BlockCache from disk.
  pub fn new(block_id: usize, block_device: Arc<dyn BlockDevice>) -> Self {
    let mut cache = [0u8; BLOCK_SZ];
    block_device.read_block(block_id, &mut cache);
    Self {
      cache,
      block_id,
      block_device,
      modified: false,
    }
  }

  /// Get the address of an offset inside the cached block data.
  fn addr_of_offset(&self, offset: usize) -> usize {
    &self.cache[offset] as *const _ as usize
  }

  /// It can get an immutable reference to an on-disk data structure of type
  /// T at offset in the buffer.
  pub fn get_ref<T>(&self, offset: usize) -> &T
  where
    T: Sized,
  {
    let type_size = core::mem::size_of::<T>();
    assert!(offset + type_size <= BLOCK_SZ);
    let addr = self.addr_of_offset(offset);
    unsafe { &*(addr as *const T) }
  }

  /// It can get an mutable reference to an on-disk data structure of type
  /// T at offset in the buffer.
  pub fn get_mut<T>(&mut self, offset: usize) -> &mut T {
    let type_size = core::mem::size_of::<T>();
    assert!(offset + type_size <= BLOCK_SZ);
    self.modified = true;
    let addr = self.addr_of_offset(offset);
    unsafe { &mut *(addr as *mut T) }
  }

  pub fn read<T, V>(&self, offset: usize, f: impl FnOnce(&T) -> V) -> V {
    f(self.get_ref(offset))
  }

  pub fn modify<T, V>(
    &mut self,
    offset: usize,
    f: impl FnOnce(&mut T) -> V,
  ) -> V {
    f(self.get_mut(offset))
  }

  /// RALL Design Thought
  ///
  /// The contents of the buffer will only be written back to disk
  /// if it has indeed been modified
  pub fn sync(&mut self) {
    if self.modified {
      self.modified = false;
      self.block_device.write_block(self.block_id, &self.cache);
    }
  }
}

impl Drop for BlockCache {
  fn drop(&mut self) {
    self.sync();
  }
}

/// Use a block cache of 16 blocks;
const BLOCK_CACHE_SIZE: usize = 16;
pub struct BlockCacheManager {
  queue: VecDeque<(usize, Arc<Mutex<BlockCache>>)>,
}

impl BlockCacheManager {
  pub fn new() -> Self {
    Self {
      queue: VecDeque::new(),
    }
  }

  pub fn get_block_cache(
    &mut self,
    block_id: usize,
    block_device: Arc<dyn BlockDevice>,
  ) -> Arc<Mutex<BlockCache>> {
    // try to find a block-cache with the same block-id.
    if let Some(pair) = self.queue.iter().find(|pair| pair.0 == block_id) {
      Arc::clone(&pair.1)
    } else {
      // substitute, throw out a block-cache. (FIFO here)
      // the head block-cache may still being used,
      // if the strong reference count less than 2 (< 2), this block-cache can be removed.
      if self.queue.len() == BLOCK_CACHE_SIZE {
        // from front to tail
        if let Some((idx, _)) = self
          .queue
          .iter()
          .enumerate()
          .find(|(_, pair)| Arc::strong_count(&pair.1) == 1)
        {
          self.queue.drain(idx..=idx);
        } else {
          // the block-caches in the self.queue are all being using, the queue is full.
          panic!("Run out of BlockCache!");
        }
      }
      // load block into memory and push it back to the self.queue, after that return to requester.
      let block_cache = Arc::new(Mutex::new(BlockCache::new(
        block_id,
        Arc::clone(&block_device),
      )));
      self.queue.push_back((block_id, Arc::clone(&block_cache)));
      block_cache
    }
  }
}
