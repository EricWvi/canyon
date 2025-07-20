use super::cache::CacheManager;
use crate::{BLOCK_BITS, BLOCK_SIZE};
use alloc::sync::Arc;

pub struct Bitmap {
    start_block_id: u64,
    blocks: u64,
    cache_manager: Arc<CacheManager>,
}

type BitmapBlock = [u64; BLOCK_SIZE as usize / 8];

impl Bitmap {
    pub fn new(start_block_id: u64, blocks: u64, cache_manager: Arc<CacheManager>) -> Self {
        Self {
            start_block_id,
            blocks,
            cache_manager,
        }
    }

    pub fn total_count(&self) -> u64 {
        self.blocks * BLOCK_BITS
    }

    pub fn free_count(&self) -> u64 {
        let mut count = 0;
        for block_id in 0..self.blocks {
            count += unsafe {
                self.cache_manager
                    .get(block_id + self.start_block_id)
                    .read()
                    .read(0, |bitmap_block: &BitmapBlock| {
                        bitmap_block
                            .iter()
                            .map(|bits64| {
                                let mut one_count = 0u64;
                                let mut num = *bits64;
                                while num != 0 {
                                    if num & 1 == 1 {
                                        one_count += 1;
                                    }
                                    num >>= 1;
                                }
                                64 - one_count
                            })
                            .sum::<u64>()
                    })
            };
        }
        count
    }

    pub fn alloc(&self) -> Option<u64> {
        for block_id in 0..self.blocks {
            let pos = unsafe {
                let cache = self.cache_manager.get(block_id + self.start_block_id);
                let mut cache = cache.write();
                let id = cache.read(0, |bitmap_block: &BitmapBlock| {
                    bitmap_block
                        .iter()
                        .enumerate()
                        .find(|(_, bits64)| **bits64 != u64::MAX)
                        .map(|(bits64_pos, bits64)| (bits64_pos, bits64.trailing_ones() as usize))
                });
                if let Some((bits64_pos, inner_pos)) = id {
                    // modify cache
                    cache.modify(0, |bitmap_block: &mut BitmapBlock| {
                        bitmap_block[bits64_pos] |= 1u64 << inner_pos;
                    });
                    Some(block_id * BLOCK_BITS + (bits64_pos * 64 + inner_pos) as u64)
                } else {
                    None
                }
            };
            if pos.is_some() {
                return pos;
            }
        }
        None
    }

    pub fn dealloc(&self, cache_manager: Arc<CacheManager>, bit: u64) {
        let (block_pos, bits64_pos, inner_pos) = decompose(bit);
        unsafe {
            cache_manager
                .get(block_pos + self.start_block_id)
                .write()
                .modify(0, |bitmap_block: &mut BitmapBlock| {
                    assert!(bitmap_block[bits64_pos] & (1u64 << inner_pos) > 0);
                    bitmap_block[bits64_pos] -= 1u64 << inner_pos;
                });
        }
    }
}

/// Return (block_pos, bits64_pos, inner_pos)
fn decompose(mut bit: u64) -> (u64, usize, u64) {
    let block_pos = bit / BLOCK_BITS;
    bit = bit % BLOCK_BITS;
    (block_pos, (bit / 64) as usize, bit % 64)
}
