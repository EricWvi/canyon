use crate::fs::{Inode, InodeType, FS};
use crate::{BlockDevice, BLOCK_BITS, BLOCK_SIZE};
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;
use bitmap::Bitmap;
use cache::CacheManager;
use core::iter;
use layout::{DataBlock, Meta, SuperBlock};
use spin::RwLock;

mod bitmap;
pub mod cache;
mod layout;

pub const NAME_LENGTH_LIMIT: usize = 199;

// padding: 8 + 8 + 8 + 8 + 8 + 8 + 184 + 24
pub struct CaInode {
    cache_manager: Arc<CacheManager>,
    inode_number: u64,
    type_: InodeType,
    size: u64,
    block_id: u64,
    offset: usize,
    name: [u8; NAME_LENGTH_LIMIT + 1],
    blocks: Vec<u64>,
}

impl CaInode {
    pub fn new(
        cache_manager: Arc<CacheManager>,
        inode_number: u64,
        type_: InodeType,
        block_id: u64,
        offset: usize,
        name: String,
    ) -> Self {
        assert!(name.len() <= NAME_LENGTH_LIMIT);
        let mut bytes = [0u8; NAME_LENGTH_LIMIT + 1];
        (&mut bytes[..name.len()]).copy_from_slice(name.as_bytes());

        unsafe {
            cache_manager
                .get(block_id)
                .write()
                .modify(offset, |meta: &mut Meta| meta.init(type_, bytes));
        }
        Self {
            cache_manager,
            inode_number,
            type_,
            size: 0,
            block_id,
            offset,
            name: bytes,
            blocks: vec![],
        }
    }

    pub fn from(
        inode_number: u64,
        block_id: u64,
        offset: usize,
        cache_manager: Arc<CacheManager>,
    ) -> Self {
        let (type_, size, (_, blocks), indirect, name) = unsafe {
            cache_manager
                .get(block_id)
                .read()
                .read(offset, |meta: &Meta| {
                    (
                        meta.type_(),
                        meta.size(),
                        meta.blocks(cache_manager.clone()),
                        meta.indirect(),
                        meta.name(),
                    )
                })
        };
        Self {
            cache_manager,
            inode_number,
            type_,
            size,
            block_id,
            offset,
            name,
            blocks,
        }
    }
}

impl Inode for CaInode {
    fn inode_number(&self) -> u64 {
        self.inode_number
    }

    fn inode_type(&self) -> InodeType {
        self.type_
    }

    fn is_file(&self) -> bool {
        self.type_ == InodeType::File
    }

    fn data(&self) -> Vec<u8> {
        let mut data = Vec::with_capacity(self.size as usize);
        if !self.blocks.is_empty() {
            let last = self.blocks.last().unwrap();
            for id in self.blocks.iter() {
                unsafe {
                    self.cache_manager
                        .get(*id)
                        .read()
                        .read(0, |block: &DataBlock| {
                            if *id == *last {
                                let last_size = if self.size % BLOCK_SIZE != 0 {
                                    self.size % BLOCK_SIZE
                                } else {
                                    BLOCK_SIZE
                                };
                                for i in block.iter().take(last_size as usize) {
                                    data.push(*i);
                                }
                            } else {
                                for i in block {
                                    data.push(*i);
                                }
                            }
                        });
                }
            }
        }
        data
    }

    fn name(&self) -> String {
        let first_zero_index = self
            .name
            .iter()
            .position(|&x| x == 0)
            .unwrap_or(self.name.len());
        String::from_utf8_lossy(&self.name[..first_zero_index]).to_string()
    }

    fn size(&self) -> u64 {
        self.size
    }
}

const INODE_CACHE_SIZE: usize = 32;

pub fn inode_number_binary(inode_number: u64) -> [u8; 10] {
    let mut repre = [0; 10];
    let bytes = inode_number.to_le_bytes();
    let mut flag = 0;
    let mut meta_flag = 10;
    for (i, byte) in bytes.iter().enumerate() {
        if *byte == 0 {
            repre[i] = u8::MAX;
            flag += 1 << i;
        } else {
            repre[i] = *byte;
        }
    }
    if flag == 0 {
        flag = u8::MAX;
        meta_flag = 20;
    }
    repre[8] = flag;
    repre[9] = meta_flag;
    repre
}

pub fn inode_number_from(arr: [u8; 10]) -> u64 {
    let meta_flag = arr[9];
    let mut flag = arr[8];
    if meta_flag == 20 {
        flag = 0;
    }
    let mut bytes = [0; 8];
    for (i, byte) in arr.iter().take(8).enumerate() {
        if (flag & (1 << i)) >> i == 0 {
            bytes[i] = *byte;
        }
    }
    u64::from_le_bytes(bytes)
}

pub struct CAFS {
    cache_manager: Arc<CacheManager>,
    inode_bitmap: Bitmap,
    data_bitmap: Bitmap,
    inode_area_start_block: u64,
    data_area_start_block: u64,
    inode_cache: RwLock<Vec<Arc<RwLock<CaInode>>>>,
}

impl Drop for CAFS {
    fn drop(&mut self) {}
}

impl CAFS {
    pub fn init(
        block_device: Arc<RwLock<dyn BlockDevice>>,
        total_blocks: u64,
        inode_bitmap_blocks: u64,
    ) -> Arc<Self> {
        assert_eq!(0, core::mem::size_of::<Meta>() % 8);

        let cache_manager = Arc::new(CacheManager::new(block_device));

        let inode_bitmap = Bitmap::new(1, inode_bitmap_blocks, cache_manager.clone());
        let inode_num = inode_bitmap.total_count();
        let inode_area_blocks =
            (inode_num * core::mem::size_of::<Meta>() as u64 + BLOCK_SIZE - 1) / BLOCK_SIZE;
        let inode_total_blocks = inode_bitmap_blocks + inode_area_blocks;

        let data_total_blocks = total_blocks - 1 - inode_total_blocks;
        let data_bitmap_blocks = data_total_blocks / (BLOCK_BITS + 1);
        let data_area_blocks = data_total_blocks - data_bitmap_blocks;
        let data_bitmap = Bitmap::new(
            1 + inode_total_blocks,
            data_bitmap_blocks,
            cache_manager.clone(),
        );
        let fs = Arc::new(Self {
            cache_manager,
            inode_bitmap: inode_bitmap,
            data_bitmap: data_bitmap,
            inode_area_start_block: 1 + inode_bitmap_blocks,
            data_area_start_block: 1 + inode_total_blocks + data_bitmap_blocks,
            inode_cache: Default::default(),
        });
        unsafe {
            // clear all blocks
            for i in 0..total_blocks {
                fs.cache_manager
                    .get(i)
                    .write()
                    .modify(0, |data_block: &mut DataBlock| {
                        for byte in data_block.iter_mut() {
                            *byte = 0;
                        }
                    });
            }
            // initialize SuperBlock
            fs.cache_manager
                .get(0)
                .write()
                .modify(0, |super_block: &mut SuperBlock| {
                    super_block.initialize(
                        total_blocks,
                        inode_bitmap_blocks,
                        inode_area_blocks,
                        data_bitmap_blocks,
                        data_area_blocks,
                    );
                });
            // create an inode for root dir "/"
            assert_eq!(
                fs.alloc_inode_meta(InodeType::Dir, "/".to_string())
                    .read()
                    .inode_number,
                0
            );
            fs.flush()
        }
        fs
    }

    pub fn open(block_device: Arc<RwLock<dyn BlockDevice>>) -> Self {
        let cache_manager = Arc::new(CacheManager::new(block_device));

        // read SuperBlock
        unsafe {
            cache_manager
                .get(0)
                .read()
                .read(0, |super_block: &SuperBlock| {
                    assert!(super_block.is_valid(), "Error loading CAFS!");
                    let inode_total_blocks =
                        super_block.inode_bitmap_blocks + super_block.inode_area_blocks;
                    let inode_bitmap =
                        Bitmap::new(1, super_block.inode_bitmap_blocks, cache_manager.clone());
                    let data_bitmap = Bitmap::new(
                        1 + inode_total_blocks,
                        super_block.data_bitmap_blocks,
                        cache_manager.clone(),
                    );

                    Self {
                        cache_manager,
                        inode_bitmap,
                        data_bitmap,
                        inode_area_start_block: 1 + super_block.inode_bitmap_blocks,
                        data_area_start_block: 1
                            + inode_total_blocks
                            + super_block.data_bitmap_blocks,
                        inode_cache: Default::default(),
                    }
                })
        }
    }

    pub fn alloc_inode_meta(&self, type_: InodeType, name: String) -> Arc<RwLock<CaInode>> {
        let id = if let Some(id) = self.inode_bitmap.alloc() {
            id
        } else {
            panic!("run out of inode block")
        };
        let (block_id, offset) = self.inode_pos_of(id);
        let meta = Arc::new(RwLock::new(CaInode::new(
            self.cache_manager.clone(),
            id,
            type_,
            block_id,
            offset,
            name,
        )));
        // add to inode_meta cache
        self.add_inode_cache(meta.clone());
        meta
    }

    pub fn alloc_data(&self) -> u64 {
        if let Some(id) = self.data_bitmap.alloc() {
            id + self.data_area_start_block
        } else {
            panic!("run out of data block")
        }
    }

    pub fn flush(&self) {
        self.cache_manager.flush()
    }

    pub fn inode_pos_of(&self, id: u64) -> (u64, usize) {
        let inode_size = core::mem::size_of::<Meta>();
        let inodes_per_block = BLOCK_SIZE / (inode_size as u64);
        let block_id = self.inode_area_start_block + id / inodes_per_block;
        (block_id, (id % inodes_per_block) as usize * inode_size)
    }

    pub fn add_inode_cache(&self, inode_meta: Arc<RwLock<CaInode>>) {
        let mut queue = self.inode_cache.write();
        if queue.len() == INODE_CACHE_SIZE {
            if let Some((idx, _)) = queue
                .iter()
                .enumerate()
                .find(|(_, cache)| Arc::strong_count(cache) == 1)
            {
                queue.swap_remove(idx);
            } else {
                panic!("Run out of InodeCache!");
            }
        }
        queue.push(inode_meta);
    }

    fn cainode(&self, inode_number: u64) -> Arc<RwLock<CaInode>> {
        let inode_cache = self.inode_cache.read();
        if let Some(cache) = inode_cache
            .iter()
            .find(|cache| cache.read().inode_number == inode_number)
        {
            cache.clone()
        } else {
            drop(inode_cache);
            let mut queue = self.inode_cache.write();
            if let Some(cache) = queue
                .iter()
                .find(|cache| cache.read().inode_number == inode_number)
            {
                cache.clone()
            } else {
                // substitute
                if queue.len() == INODE_CACHE_SIZE {
                    if let Some((idx, _)) = queue
                        .iter()
                        .enumerate()
                        .find(|(_, cache)| Arc::strong_count(cache) == 1)
                    {
                        queue.swap_remove(idx);
                    } else {
                        panic!("Run out of InodeCache!");
                    }
                }
                let (block_id, offset) = self.inode_pos_of(inode_number);
                let inode_cache = Arc::new(RwLock::new(CaInode::from(
                    inode_number,
                    block_id,
                    offset,
                    self.cache_manager.clone(),
                )));
                queue.push(inode_cache.clone());
                inode_cache
            }
        }
    }
}

impl FS for CAFS {
    fn create(&self, parent: u64, name: String) -> Arc<RwLock<dyn Inode>> {
        let meta = self.alloc_inode_meta(InodeType::File, name);

        // add to dir data block
        let parent = self.inode(parent);
        let parent_inode_number = parent.read().inode_number();
        let mut contents = parent.read().data();
        if !contents.is_empty() {
            contents.push(0);
        }
        for i in inode_number_binary(meta.read().inode_number()) {
            contents.push(i);
        }
        self.write(parent_inode_number, &contents);

        // add to inode cache
        self.add_inode_cache(meta.clone());
        meta
    }

    fn write(&self, inode_number: u64, contents: &Vec<u8>) {
        let new_size = contents.len() as u64;
        let inode = self.cainode(inode_number);
        let mut inode = inode.write();
        let (block_id, offset) = self.inode_pos_of(inode_number);
        unsafe {
            let (_, blocks) =
                self.cache_manager
                    .get(block_id)
                    .write()
                    .modify(offset, |meta: &mut Meta| {
                        let curr_info = Meta::index_blocks(meta.size());
                        let new_info = Meta::index_blocks(new_size);
                        if meta.size() < new_size {
                            let mut data_blocks = vec![];
                            let mut index_blocks = vec![];
                            for _ in 0..(Meta::_data_blocks(new_size) - meta.data_blocks()) {
                                data_blocks.push(self.alloc_data());
                            }
                            for _ in
                                0..(new_info.index_block_count() - curr_info.index_block_count())
                            {
                                index_blocks.push(self.alloc_data());
                            }
                            meta.extend(
                                new_size,
                                new_info,
                                data_blocks,
                                index_blocks,
                                self.cache_manager.clone(),
                            );
                        } else if meta.size() > new_size {
                            let ids = meta.shrink(new_size, self.cache_manager.clone());
                            for id in ids.0 {
                                self.data_bitmap.dealloc(self.cache_manager.clone(), id);
                            }
                            for id in ids.1 {
                                self.data_bitmap.dealloc(self.cache_manager.clone(), id);
                            }
                        }
                        meta.blocks(self.cache_manager.clone())
                    });
            let mut contents = contents.iter().chain(iter::repeat(&0));
            for id in &blocks {
                self.cache_manager
                    .get(*id)
                    .write()
                    .modify(0, |block: &mut DataBlock| {
                        for i in block.iter_mut() {
                            *i = *(contents.next().unwrap());
                        }
                    })
            }
            inode.size = new_size;
            inode.blocks = blocks;
        }
    }

    fn df(&self) -> (u64, u64) {
        let free = self.data_bitmap.free_count();
        let total = self.data_bitmap.total_count();
        (free * BLOCK_SIZE, total * BLOCK_SIZE)
    }

    fn inode(&self, inode_number: u64) -> Arc<RwLock<dyn Inode>> {
        self.cainode(inode_number)
    }

    fn sub_inodes(&self, inode_number: u64) -> Vec<u64> {
        let mut inodes = vec![];
        let parent = self.inode(inode_number);
        if parent.read().is_file() {
            return inodes;
        }
        let data = parent.read().data();
        if data.is_empty() {
            return inodes;
        }
        for sub_inode in data.split(|num| *num == 0) {
            let mut arr = [0u8; 10];
            arr.copy_from_slice(&sub_inode[..10]);
            let inode_number = inode_number_from(arr);
            inodes.push(inode_number);
        }
        inodes
    }
}

#[cfg(test)]
mod test {
    use crate::cafs::{CAFS, FS};
    use crate::fake::Disk;
    use crate::BLOCK_SIZE;
    use spin::RwLock;
    use std::sync::Arc;

    fn fake_fs() -> Arc<CAFS> {
        let total_blocks = 20 << 10;
        let inode_bitmap_blocks = 2;
        let disk = Disk::new(total_blocks);
        CAFS::init(
            Arc::new(RwLock::new(disk)),
            total_blocks,
            inode_bitmap_blocks,
        )
    }

    #[test]
    fn test_fs_init() {}

    #[test]
    fn test_inode_meta_data() {
        let contents = (0..u32::MAX >> 10)
            .flat_map(|x| x.to_le_bytes())
            .collect::<Vec<_>>();

        let total_blocks = 128 << 10;
        let inode_bitmap_blocks = 2;
        let disk = Disk {
            total_blocks,
            data: vec![[0; BLOCK_SIZE as usize]; total_blocks as usize],
        };
        let fs = CAFS::init(
            Arc::new(RwLock::new(disk)),
            total_blocks,
            inode_bitmap_blocks,
        );
        let meta = fs.create(0, "test.txt".to_string());
        let inode_number = meta.read().inode_number();
        fs.write(inode_number, &contents);
        assert_eq!(meta.read().data(), contents);
    }
}
