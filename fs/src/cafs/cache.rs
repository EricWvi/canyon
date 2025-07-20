use crate::BlockDevice;
use crate::BLOCK_SIZE;
use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::RwLock;

const CACHE_SIZE: usize = 32;

pub struct Cache {
    cache: [u8; BLOCK_SIZE as usize],
    block_id: u64,
    block_device: Arc<RwLock<dyn BlockDevice>>,
    modified: bool,
}

impl Cache {
    /// Load a new BlockCache from disk.
    pub fn new(block_id: u64, block_device: Arc<RwLock<dyn BlockDevice>>) -> Self {
        let mut cache = [0u8; BLOCK_SIZE as usize];
        block_device.read().read_block(block_id, &mut cache);
        Self {
            cache,
            block_id,
            block_device,
            modified: false,
        }
    }

    fn offset_addr(&self, offset: usize) -> usize {
        &self.cache[offset] as *const _ as usize
    }

    pub unsafe fn get_ref<T>(&self, offset: usize) -> &T
    where
        T: Sized,
    {
        let type_size = core::mem::size_of::<T>();
        assert!(offset + type_size <= BLOCK_SIZE as usize);
        let addr = self.offset_addr(offset);
        &*(addr as *const T)
    }

    pub unsafe fn get_mut<T>(&mut self, offset: usize) -> &mut T
    where
        T: Sized,
    {
        let type_size = core::mem::size_of::<T>();
        assert!(offset + type_size <= BLOCK_SIZE as usize);
        self.modified = true;
        let addr = self.offset_addr(offset);
        &mut *(addr as *mut T)
    }

    pub fn sync(&mut self) {
        if self.modified {
            self.modified = false;
            self.block_device
                .write()
                .write_block(self.block_id, &self.cache);
        }
    }

    pub unsafe fn read<T, V>(&self, offset: usize, f: impl FnOnce(&T) -> V) -> V {
        f(self.get_ref(offset))
    }

    pub unsafe fn modify<T, V>(&mut self, offset: usize, f: impl FnOnce(&mut T) -> V) -> V {
        f(self.get_mut(offset))
    }
}

impl Drop for Cache {
    fn drop(&mut self) {
        self.sync()
    }
}

pub struct CacheManager {
    queue: RwLock<Vec<(u64, Arc<RwLock<Cache>>)>>,
    block_device: Arc<RwLock<dyn BlockDevice>>,
}

impl CacheManager {
    pub fn new(block_device: Arc<RwLock<dyn BlockDevice>>) -> Self {
        Self {
            queue: RwLock::new(Vec::with_capacity(CACHE_SIZE)),
            block_device,
        }
    }

    pub fn get(&self, block_id: u64) -> Arc<RwLock<Cache>> {
        let lock = self.queue.read();
        if let Some((_, cache)) = lock.iter().find(|(id, _)| *id == block_id) {
            cache.clone()
        } else {
            drop(lock);
            let mut queue = self.queue.write();
            if let Some((_, cache)) = queue.iter().find(|(id, _)| *id == block_id) {
                cache.clone()
            } else {
                // substitute
                if queue.len() == CACHE_SIZE {
                    if let Some((idx, _)) = queue
                        .iter()
                        .enumerate()
                        .find(|(_, (_, cache))| Arc::strong_count(cache) == 1)
                    {
                        queue.swap_remove(idx);
                    } else {
                        panic!("Run out of BlockCache!");
                    }
                }
                let block_cache =
                    Arc::new(RwLock::new(Cache::new(block_id, self.block_device.clone())));
                queue.push((block_id, block_cache.clone()));
                block_cache
            }
        }
    }

    pub fn flush(&self) {
        for (_, cache) in &*self.queue.read() {
            cache.write().sync();
        }
    }
}
