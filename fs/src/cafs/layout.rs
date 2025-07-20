use super::cache::CacheManager;
use crate::fs::InodeType;
use crate::BLOCK_SIZE;
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;

const FS_MAGIC: u32 = 0x5138;

#[repr(C)]
pub struct SuperBlock {
    magic: u32,
    pub total_blocks: u64,
    pub inode_bitmap_blocks: u64,
    pub inode_area_blocks: u64,
    pub data_bitmap_blocks: u64,
    pub data_area_blocks: u64,
}

impl SuperBlock {
    pub fn initialize(
        &mut self,
        total_blocks: u64,
        inode_bitmap_blocks: u64,
        inode_area_blocks: u64,
        data_bitmap_blocks: u64,
        data_area_blocks: u64,
    ) {
        *self = Self {
            magic: FS_MAGIC,
            total_blocks,
            inode_bitmap_blocks,
            inode_area_blocks,
            data_bitmap_blocks,
            data_area_blocks,
        }
    }
    pub fn is_valid(&self) -> bool {
        self.magic == FS_MAGIC
    }
}

const DIRECT_COUNT: usize = 36;
// size: 8 + 8 * 36 + 8 + 8 + 200
#[repr(C)]
pub struct Meta {
    size: u64,
    direct: [u64; DIRECT_COUNT],
    indirect: u64,
    type_: InodeType,
    name: [u8; 199 + 1],
}

impl Meta {
    pub fn init(&mut self, type_: InodeType, name: [u8; 200]) {
        self.size = 0;
        self.direct.iter_mut().for_each(|v| *v = 0);
        self.indirect = 0;
        self.type_ = type_;
        self.name = name;
    }

    pub fn size(&self) -> u64 {
        self.size
    }

    pub fn type_(&self) -> InodeType {
        self.type_
    }

    pub fn direct(&self) -> &[u64] {
        &self.direct[..]
    }

    pub fn indirect(&self) -> u64 {
        self.indirect
    }

    pub fn name(&self) -> [u8; 200] {
        self.name
    }

    pub fn is_dir(&self) -> bool {
        self.type_ == InodeType::Dir
    }

    pub fn is_file(&self) -> bool {
        self.type_ == InodeType::File
    }

    pub fn get_block_id(&self, inner_id: u64, cache_manager: Arc<CacheManager>) -> Option<u64> {
        unsafe {
            if ((self.size + BLOCK_SIZE - 1) / BLOCK_SIZE) <= inner_id {
                None
            } else if inner_id < DIRECT_COUNT as u64 {
                Some(self.direct[inner_id as usize])
            } else {
                let cache = cache_manager.get(self.indirect);
                let cache = cache.read();
                Some(
                    cache
                        .get_ref::<IndirectBlock>(0)
                        .get_block_id(inner_id - DIRECT_COUNT as u64, cache_manager),
                )
            }
        }
    }

    /// return (index, blocks)
    pub fn blocks(&self, cache_manager: Arc<CacheManager>) -> (Vec<u64>, Vec<u64>) {
        let mut blocks = self
            .direct
            .iter()
            .filter(|x| **x != 0)
            .map(|x| *x)
            .collect::<Vec<_>>();
        let mut index = vec![];
        if self.indirect != 0 {
            index.push(self.indirect);
            let (mut index_ids, mut data_ids) = unsafe {
                cache_manager
                    .get(self.indirect)
                    .read()
                    .read(0, |block: &IndirectBlock| {
                        block.to_vec(cache_manager.clone(), None)
                    })
            };
            blocks.append(&mut data_ids);
            index.append(&mut index_ids);
        }
        index.sort();
        (index, blocks)
    }

    /// Return block number correspond to size.
    pub fn data_blocks(&self) -> u64 {
        Self::_data_blocks(self.size)
    }

    pub fn _data_blocks(size: u64) -> u64 {
        (size + BLOCK_SIZE - 1) / BLOCK_SIZE
    }

    /// Return number of blocks needed including indirect block
    pub fn index_blocks(size: u64) -> LevelInfo {
        let indirect_size = if size >= DIRECT_MAX {
            size - DIRECT_MAX
        } else {
            0
        };
        let block_table_len =
            (indirect_size + BLOCK_TABLE_INDIRECT_MAX - 1) / BLOCK_TABLE_INDIRECT_MAX;
        let block_directory_len =
            (indirect_size + BLOCK_DIRECTORY_INDIRECT_MAX - 1) / BLOCK_DIRECTORY_INDIRECT_MAX;
        let l3_len = (indirect_size + L3_INDIRECT_MAX - 1) / L3_INDIRECT_MAX;
        let (l4, l3, block_directory, block_table, direct) = if size > L3_MAX {
            (
                1,
                l3_len,
                block_directory_len,
                block_table_len,
                DIRECT_COUNT as u64,
            )
        } else if size > BLOCK_DIRECTORY_MAX {
            (
                0,
                1,
                block_directory_len,
                block_table_len,
                DIRECT_COUNT as u64,
            )
        } else if size > BLOCK_TABLE_MAX {
            (0, 0, 1, block_table_len, DIRECT_COUNT as u64)
        } else if size > DIRECT_MAX {
            (0, 0, 0, 1, DIRECT_COUNT as u64)
        } else {
            (0, 0, 0, 0, (size + BLOCK_SIZE - 1) / BLOCK_SIZE)
        };
        LevelInfo {
            l4,
            l3,
            block_directory,
            block_table,
            direct,
        }
    }

    pub fn forward(
        &mut self,
        level_info: LevelInfo,
        index: Vec<u64>,
        data: Vec<u64>,
        cache_manager: Arc<CacheManager>,
    ) {
        let mut index_iter = index.into_iter().chain(core::iter::repeat(0));
        let mut data_iter = data.into_iter().chain(core::iter::repeat(0));

        self.direct = data_iter
            .by_ref()
            .take(DIRECT_COUNT)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

        let curr_level = level_info.root_level();

        let indirect_size = if self.size >= DIRECT_MAX {
            self.size - DIRECT_MAX
        } else {
            0
        };
        if indirect_size != 0 {
            let l4_id = if level_info.l4 != 0 {
                index_iter.next().unwrap()
            } else {
                0
            };
            let l3_ids = index_iter
                .by_ref()
                .take(level_info.l3 as usize)
                .collect::<Vec<_>>();
            let l3_id = *l3_ids.first().unwrap_or(&0);
            let l2_ids = index_iter
                .by_ref()
                .take(level_info.block_directory as usize)
                .collect::<Vec<_>>();
            let l2_id = *l2_ids.first().unwrap_or(&0);
            let l1_ids = index_iter
                .by_ref()
                .take(level_info.block_table as usize)
                .collect::<Vec<_>>();
            let l1_id = *l1_ids.first().unwrap_or(&0);

            if l4_id != 0 {
                unsafe {
                    cache_manager
                        .get(l4_id)
                        .write()
                        .modify(0, |block: &mut IndirectBlock| {
                            block.type_ = IndirectBlockType::L4;
                            block.entries = l3_ids
                                .iter()
                                .map(|x| *x)
                                .chain(core::iter::repeat(0))
                                .take(INDIRECT_LEN)
                                .collect::<Vec<_>>()
                                .try_into()
                                .unwrap();
                        });
                }
            }
            let mut l3_entries = l2_ids.iter().map(|x| *x).chain(core::iter::repeat(0));
            for i in &l3_ids {
                unsafe {
                    cache_manager
                        .get(*i)
                        .write()
                        .modify(0, |block: &mut IndirectBlock| {
                            block.type_ = IndirectBlockType::L3;
                            block.entries = l3_entries
                                .by_ref()
                                .take(INDIRECT_LEN)
                                .collect::<Vec<_>>()
                                .try_into()
                                .unwrap();
                        });
                }
            }
            let mut l2_entries = l1_ids.iter().map(|x| *x).chain(core::iter::repeat(0));
            for i in &l2_ids {
                unsafe {
                    cache_manager
                        .get(*i)
                        .write()
                        .modify(0, |block: &mut IndirectBlock| {
                            block.type_ = IndirectBlockType::BlockDirectory;
                            block.entries = l2_entries
                                .by_ref()
                                .take(INDIRECT_LEN)
                                .collect::<Vec<_>>()
                                .try_into()
                                .unwrap();
                        });
                }
            }
            for i in &l1_ids {
                unsafe {
                    cache_manager
                        .get(*i)
                        .write()
                        .modify(0, |block: &mut IndirectBlock| {
                            block.type_ = IndirectBlockType::BlockTable;
                            block.entries = data_iter
                                .by_ref()
                                .take(INDIRECT_LEN)
                                .collect::<Vec<_>>()
                                .try_into()
                                .unwrap();
                        });
                }
            }

            if level_info.l4 == 1 {
                self.indirect = l4_id;
            } else if level_info.l3 == 1 {
                self.indirect = l3_id;
            } else if level_info.block_directory == 1 {
                self.indirect = l2_id;
            } else if level_info.block_table == 1 {
                self.indirect = l1_id;
            }
        } else {
            self.indirect = 0;
        }
    }

    /// # Panic
    /// panics if new_size < self.size
    pub fn extend(
        &mut self,
        new_size: u64,
        new_info: LevelInfo,
        mut data_blocks: Vec<u64>,
        mut index_blocks: Vec<u64>,
        cache_manager: Arc<CacheManager>,
    ) {
        assert!(new_size >= self.size);
        self.size = new_size;

        let (mut index, mut data) = self.blocks(cache_manager.clone());
        index.append(&mut index_blocks);
        data.append(&mut data_blocks);

        self.forward(new_info, index, data, cache_manager);
    }

    /// # Panic
    /// panics if new_size > self.size
    pub fn shrink(
        &mut self,
        new_size: u64,
        cache_manager: Arc<CacheManager>,
    ) -> (Vec<u64>, Vec<u64>) {
        assert!(new_size <= self.size);
        let prev_data_blocks = self.data_blocks();
        let curr_info = Self::index_blocks(self.size);
        let new_info = Self::index_blocks(new_size);
        self.size = new_size;

        let (mut index, mut data) = self.blocks(cache_manager.clone());
        let mut collected_index_ids = vec![];
        let mut collected_data_ids = vec![];
        for i in 0..(curr_info.index_block_count() - new_info.index_block_count()) {
            collected_index_ids.push(index.pop().unwrap());
        }
        for i in 0..(prev_data_blocks - Meta::_data_blocks(new_size)) {
            collected_data_ids.push(data.pop().unwrap());
        }

        self.forward(new_info, index, data, cache_manager);

        (collected_index_ids, collected_data_ids)
    }

    /// Clear size to zero and return blocks that should be deallocated.
    pub fn clear_size(&mut self, cache_manager: Arc<CacheManager>) -> Vec<u64> {
        let (mut index, mut data) = self.blocks(cache_manager);
        self.init(self.type_, self.name);
        index.append(&mut data);
        index
    }
}

// num of entries in a indirect block = 511
const INDIRECT_LEN: usize = (BLOCK_SIZE / 8) as usize - 1;
// ~ 18 KB
const DIRECT_MAX: u64 = DIRECT_COUNT as u64 * BLOCK_SIZE;

const BLOCK_TABLE_INDIRECT_MAX: u64 = INDIRECT_LEN as u64 * BLOCK_SIZE;
// ~ 50 KB
const BLOCK_TABLE_MAX: u64 = DIRECT_MAX + BLOCK_TABLE_INDIRECT_MAX;

const BLOCK_DIRECTORY_INDIRECT_MAX: u64 = (INDIRECT_LEN * INDIRECT_LEN) as u64 * BLOCK_SIZE;
// ~ 2 MB
const BLOCK_DIRECTORY_MAX: u64 = DIRECT_MAX + BLOCK_DIRECTORY_INDIRECT_MAX;

const L3_INDIRECT_MAX: u64 = (INDIRECT_LEN * INDIRECT_LEN * INDIRECT_LEN) as u64 * BLOCK_SIZE;
// ~ 122 MB
const L3_MAX: u64 = DIRECT_MAX + L3_INDIRECT_MAX;

const L4_INDIRECT_MAX: u64 =
    (INDIRECT_LEN * INDIRECT_LEN * INDIRECT_LEN * INDIRECT_LEN) as u64 * BLOCK_SIZE;
// ~ 7 GB
const L4_MAX: u64 = DIRECT_MAX + L4_INDIRECT_MAX;

#[derive(Debug, Eq, PartialEq)]
pub struct LevelInfo {
    pub l4: u64,
    pub l3: u64,
    pub block_directory: u64,
    pub block_table: u64,
    pub direct: u64,
}

impl LevelInfo {
    fn root_level(&self) -> Option<IndirectBlockType> {
        if self.l4 == 1 {
            return Some(IndirectBlockType::L4);
        }
        if self.l3 == 1 {
            return Some(IndirectBlockType::L3);
        }
        if self.block_directory == 1 {
            return Some(IndirectBlockType::BlockDirectory);
        }
        if self.block_table == 1 {
            return Some(IndirectBlockType::BlockTable);
        }
        None
    }

    fn type_count(&self, type_: IndirectBlockType) -> u64 {
        match type_ {
            IndirectBlockType::BlockTable => self.block_table,
            IndirectBlockType::BlockDirectory => self.block_directory,
            IndirectBlockType::L3 => self.l3,
            IndirectBlockType::L4 => self.l4,
        }
    }

    pub fn index_block_count(&self) -> u64 {
        return self.l4 + self.l3 + self.block_directory + self.block_table;
    }
}

#[repr(C)]
pub struct IndirectBlock {
    pub entries: [u64; INDIRECT_LEN],
    pub type_: IndirectBlockType,
}

#[derive(PartialEq, Copy, Clone, Debug)]
#[repr(u64)]
pub enum IndirectBlockType {
    BlockTable,
    BlockDirectory,
    L3,
    L4,
}

impl IndirectBlockType {
    pub fn add(&self) -> Self {
        if *self as u64 == Self::L4 as u64 {
            return Self::L4;
        }
        unsafe { *(&(*self as u64 + 1) as *const u64 as *const Self) }
    }

    pub fn decrease(&self) -> Self {
        if *self as u64 == Self::BlockTable as u64 {
            return Self::BlockTable;
        }
        unsafe { *(&(*self as u64 - 1) as *const u64 as *const Self) }
    }
}

impl<'block> IndirectBlock {
    fn read(&self, cache_manager: &CacheManager) {}

    fn get_block_id(&self, inner_id: u64, cache_manager: Arc<CacheManager>) -> u64 {
        unsafe {
            match self.type_ {
                IndirectBlockType::BlockTable => self.entries[inner_id as usize],
                dir => {
                    let divisor = match dir {
                        IndirectBlockType::BlockTable => unreachable!(),
                        IndirectBlockType::BlockDirectory => INDIRECT_LEN as u64,
                        IndirectBlockType::L3 => (INDIRECT_LEN * INDIRECT_LEN) as u64,
                        IndirectBlockType::L4 => {
                            (INDIRECT_LEN * INDIRECT_LEN * INDIRECT_LEN) as u64
                        }
                    };
                    let index = inner_id / divisor;
                    let offset = inner_id % divisor;
                    cache_manager
                        .get(self.entries[index as usize])
                        .read()
                        .get_ref::<IndirectBlock>(0)
                        .get_block_id(offset, cache_manager)
                }
            }
        }
    }

    pub fn to_vec(
        &self,
        cache_manager: Arc<CacheManager>,
        filter: Option<&Vec<u64>>,
    ) -> (Vec<u64>, Vec<u64>) {
        let mut data_ids = Vec::new();
        let mut index_ids = Vec::new();
        self._to_vec(&mut data_ids, &mut index_ids, cache_manager.clone(), filter);
        (index_ids, data_ids)
    }

    fn _to_vec(
        &self,
        data_ids: &mut Vec<u64>,
        index_ids: &mut Vec<u64>,
        cache_manager: Arc<CacheManager>,
        filter: Option<&Vec<u64>>,
    ) {
        unsafe {
            match self.type_ {
                IndirectBlockType::BlockTable => {
                    for id in self.entries.iter().filter(|x| {
                        **x != 0
                            && (filter.is_none()
                                || (filter.is_some() && !filter.unwrap().contains(*x)))
                    }) {
                        data_ids.push(*id)
                    }
                }
                _ => {
                    for id in self.entries.iter().filter(|x| {
                        **x != 0
                            && (filter.is_none()
                                || (filter.is_some() && !filter.unwrap().contains(*x)))
                    }) {
                        index_ids.push(*id);
                        cache_manager
                            .get(*id)
                            .read()
                            .get_ref::<IndirectBlock>(0)
                            ._to_vec(data_ids, index_ids, cache_manager.clone(), filter);
                    }
                }
            }
        }
    }
}

pub type DataBlock = [u8; BLOCK_SIZE as usize];

#[cfg(test)]
mod test {
    extern crate std;

    use super::{
        IndirectBlock, IndirectBlockType, InodeType, LevelInfo, Meta, BLOCK_DIRECTORY_INDIRECT_MAX,
        BLOCK_DIRECTORY_MAX, BLOCK_TABLE_INDIRECT_MAX, BLOCK_TABLE_MAX, DIRECT_COUNT, DIRECT_MAX,
        INDIRECT_LEN, L3_INDIRECT_MAX, L3_MAX, L4_MAX,
    };
    use alloc::vec;
    use alloc::vec::Vec;
    use spin::rwlock::RwLock;
    use std::collections::HashMap;
    use std::ops::Range;
    use std::println;
    use std::ptr::slice_from_raw_parts;
    use std::sync::Arc;

    use super::super::cache::CacheManager;
    use crate::{BlockDevice, BLOCK_SIZE};

    #[derive(Debug)]
    pub struct FakeDisk {
        pub total_blocks: u64,
        pub data: HashMap<u64, [u8; BLOCK_SIZE as usize]>,
    }

    impl FakeDisk {
        pub fn new(total_blocks: u64) -> Self {
            Self {
                total_blocks,
                data: HashMap::new(),
            }
        }
    }

    impl BlockDevice for FakeDisk {
        fn read_block(&self, block_id: u64, buf: &mut [u8]) {
            assert!(block_id < self.total_blocks);
            assert_eq!(buf.len(), BLOCK_SIZE as usize);
            if self.data.contains_key(&block_id) {
                buf.copy_from_slice(&self.data[&block_id]);
            } else {
                buf.copy_from_slice(&[0u8; BLOCK_SIZE as usize]);
            }
        }

        fn write_block(&mut self, block_id: u64, buf: &[u8]) {
            assert!(block_id < self.total_blocks);
            assert_eq!(buf.len(), BLOCK_SIZE as usize);
            self.data.insert(block_id, buf.try_into().unwrap());
        }
    }

    #[repr(C)]
    pub struct MetaTest {
        size: u64,
        direct: [u64; DIRECT_COUNT],
        indirect: u64,
        type_: InodeType,
        name: [u8; 199 + 1],
    }

    /// return (inode, index_ids, ids, cache_manager)
    fn fake_inode(size: u64) -> (Meta, Vec<u64>, Vec<u64>, Arc<CacheManager>, Range<u64>) {
        let mut fake_disk = FakeDisk::new(L4_MAX / BLOCK_SIZE);

        let mut id_iter = 100..3_000_000_000;

        let indirect_size = if size >= DIRECT_MAX {
            size - DIRECT_MAX
        } else {
            0
        };
        let (root, mut index_ids, mut block_ids) = if indirect_size != 0 {
            // L4
            let l3_counts = (indirect_size + L3_INDIRECT_MAX - 1) / L3_INDIRECT_MAX;
            let mut l3_indexes = (&mut id_iter).take(l3_counts as usize).collect::<Vec<_>>();
            let l4_entries = l3_indexes
                .clone()
                .into_iter()
                .chain(core::iter::repeat(0))
                .take(INDIRECT_LEN)
                .collect::<Vec<_>>();
            let l3_root = l4_entries[0];
            let l4 = IndirectBlock {
                entries: l4_entries.clone().try_into().unwrap(),
                type_: IndirectBlockType::L4,
            };
            let l4_root = 11;
            let addr = &l4 as *const IndirectBlock as *const u8;
            fake_disk.write_block(l4_root, unsafe {
                &*slice_from_raw_parts(addr, BLOCK_SIZE as usize)
            });

            // L3
            let block_directory_counts =
                (indirect_size + BLOCK_DIRECTORY_INDIRECT_MAX - 1) / BLOCK_DIRECTORY_INDIRECT_MAX;
            let mut block_directory_indexes = (&mut id_iter)
                .take(block_directory_counts as usize)
                .collect::<Vec<_>>();
            let block_directory_root = block_directory_indexes[0];
            let mut l3_entries = block_directory_indexes
                .clone()
                .into_iter()
                .chain(core::iter::repeat(0));

            // block_directory
            let block_table_counts =
                (indirect_size + BLOCK_TABLE_INDIRECT_MAX - 1) / BLOCK_TABLE_INDIRECT_MAX;
            let mut block_table_indexes = (&mut id_iter)
                .take(block_table_counts as usize)
                .collect::<Vec<_>>();
            let block_table_root = block_table_indexes[0];
            let mut block_directory_entries = block_table_indexes
                .clone()
                .into_iter()
                .chain(core::iter::repeat(0));

            // block_table
            let blocks_count = (indirect_size + BLOCK_SIZE - 1) / BLOCK_SIZE;
            let block_table_entries_ids = (&mut id_iter)
                .take(blocks_count as usize)
                .collect::<Vec<_>>();
            let block_ids = block_table_entries_ids.clone();
            let mut blocks = block_table_entries_ids
                .into_iter()
                .chain(core::iter::repeat(0));

            for l4_entry in l4_entries {
                if l4_entry == 0 {
                    break;
                }

                let l3_entries_ids = (&mut l3_entries).take(INDIRECT_LEN).collect::<Vec<_>>();

                for l3_entry in &l3_entries_ids {
                    let block_directory_entries_ids = (&mut block_directory_entries)
                        .take(INDIRECT_LEN)
                        .collect::<Vec<_>>();

                    for block_directory_entry in &block_directory_entries_ids {
                        let block_table_entries_ids =
                            (&mut blocks).take(INDIRECT_LEN).collect::<Vec<_>>();

                        let block_table = IndirectBlock {
                            entries: block_table_entries_ids.try_into().unwrap(),
                            type_: IndirectBlockType::BlockTable,
                        };
                        let addr = &block_table as *const IndirectBlock as *const u8;
                        fake_disk.write_block(*block_directory_entry, unsafe {
                            &*slice_from_raw_parts(addr, BLOCK_SIZE as usize)
                        });
                    }

                    let block_directory = IndirectBlock {
                        entries: block_directory_entries_ids.try_into().unwrap(),
                        type_: IndirectBlockType::BlockDirectory,
                    };
                    let addr = &block_directory as *const IndirectBlock as *const u8;
                    fake_disk.write_block(*l3_entry, unsafe {
                        &*slice_from_raw_parts(addr, BLOCK_SIZE as usize)
                    });
                }

                let l3 = IndirectBlock {
                    entries: l3_entries_ids.try_into().unwrap(),
                    type_: IndirectBlockType::L3,
                };
                let addr = &l3 as *const IndirectBlock as *const u8;
                fake_disk.write_block(l4_entry, unsafe {
                    &*slice_from_raw_parts(addr, BLOCK_SIZE as usize)
                });
            }
            let (root, indexes) = if let Some(level) = Meta::index_blocks(size).root_level() {
                let mut indexes = vec![];
                match level {
                    IndirectBlockType::BlockTable => {
                        indexes.push(block_table_root);
                        (block_table_root, indexes)
                    }
                    IndirectBlockType::BlockDirectory => {
                        indexes.push(block_directory_root);
                        indexes.append(&mut block_table_indexes);
                        (block_directory_root, indexes)
                    }
                    IndirectBlockType::L3 => {
                        indexes.push(l3_root);
                        indexes.append(&mut block_directory_indexes);
                        indexes.append(&mut block_table_indexes);
                        (l3_root, indexes)
                    }
                    IndirectBlockType::L4 => {
                        indexes.push(l4_root);
                        indexes.append(&mut l3_indexes);
                        indexes.append(&mut block_directory_indexes);
                        indexes.append(&mut block_table_indexes);
                        (l4_root, indexes)
                    }
                }
            } else {
                (0, vec![])
            };
            (root, indexes, block_ids)
        } else {
            (0, vec![], vec![])
        };

        let direct_size = if root != 0 {
            DIRECT_COUNT as u64
        } else {
            (size + BLOCK_SIZE - 1) / BLOCK_SIZE
        };

        let inode: Meta = unsafe {
            std::mem::transmute(MetaTest {
                size,
                direct: (1..=direct_size)
                    .into_iter()
                    .chain(core::iter::repeat(0))
                    .take(DIRECT_COUNT)
                    .collect::<Vec<_>>()
                    .try_into()
                    .unwrap(),
                indirect: root,
                type_: InodeType::File,
                name: [0; 200],
            })
        };
        let mut ids = inode.direct.to_vec();
        ids.append(&mut block_ids);

        let cache_manager = Arc::new(CacheManager::new(Arc::new(RwLock::new(fake_disk))));
        index_ids.sort();
        (inode, index_ids, ids, cache_manager, id_iter)
    }

    #[test]
    fn test_small_get_block_id() {
        let (small, index_ids, block_ids, cache_manager, _) = fake_inode(BLOCK_SIZE + 10);

        assert_eq!(
            small.get_block_id(0, cache_manager.clone()),
            Some(block_ids[0])
        );
        assert_eq!(
            small.get_block_id(1, cache_manager.clone()),
            Some(block_ids[1])
        );
        assert_eq!(small.get_block_id(2, cache_manager.clone()), None);
    }

    #[test]
    fn test_medium_get_block_id() {
        let (medium, index_ids, block_ids, cache_manager, _) = fake_inode(DIRECT_MAX * 10);

        assert_eq!(
            medium.get_block_id((DIRECT_COUNT - 1) as u64, cache_manager.clone()),
            Some(block_ids[DIRECT_COUNT - 1])
        );
        assert_eq!(
            medium.get_block_id(DIRECT_COUNT as u64, cache_manager.clone()),
            Some(block_ids[DIRECT_COUNT])
        );
        assert_eq!(
            medium.get_block_id(DIRECT_COUNT as u64 + 1, cache_manager.clone()),
            Some(block_ids[DIRECT_COUNT + 1])
        );
        assert_eq!(
            medium.get_block_id((block_ids.len() - 1) as u64, cache_manager.clone()),
            Some(block_ids[block_ids.len() - 1])
        );
        assert_eq!(
            medium.get_block_id(block_ids.len() as u64, cache_manager.clone()),
            None
        );
        assert_eq!(
            medium.get_block_id((block_ids.len() + 1) as u64, cache_manager.clone()),
            None
        );
    }

    #[test]
    fn test_large_get_block_id() {
        let (inode, index_ids, block_ids, cache_manager, _) =
            fake_inode(L3_MAX + BLOCK_DIRECTORY_MAX);

        assert_eq!(
            inode.get_block_id(100u64, cache_manager.clone()),
            Some(block_ids[100])
        );
        assert_eq!(
            inode.get_block_id(10000u64, cache_manager.clone()),
            Some(block_ids[10000])
        );
        assert_eq!(
            inode.get_block_id((block_ids.len() - 1) as u64, cache_manager.clone()),
            Some(*block_ids.last().unwrap())
        );
        assert_eq!(
            inode.get_block_id(block_ids.len() as u64, cache_manager.clone()),
            None
        );
    }

    #[test]
    fn test_inode_blocks() {
        let (small, index_ids_small, block_ids_small, cache_manager_small, _) =
            fake_inode(DIRECT_MAX);
        let (medium, index_ids_medium, block_ids_medium, cache_manager_medium, _) =
            fake_inode(BLOCK_DIRECTORY_MAX - BLOCK_SIZE);
        let (large, index_ids_large, block_ids_large, cache_manager_large, _) =
            fake_inode(2 * BLOCK_DIRECTORY_MAX);

        assert_eq!(
            small.blocks(cache_manager_small),
            (index_ids_small, block_ids_small)
        );

        let blocks = medium.blocks(cache_manager_medium);
        // blocks.sort();
        assert_eq!(blocks, (index_ids_medium, block_ids_medium));

        let blocks = large.blocks(cache_manager_large);
        // blocks.sort();
        assert_eq!(blocks, (index_ids_large, block_ids_large));
    }

    #[test]
    fn test_index_blocks() {
        assert_eq!(
            Meta::index_blocks(0),
            LevelInfo {
                l4: 0,
                l3: 0,
                block_directory: 0,
                block_table: 0,
                direct: 0,
            }
        );
        assert_eq!(
            Meta::index_blocks(10),
            LevelInfo {
                l4: 0,
                l3: 0,
                block_directory: 0,
                block_table: 0,
                direct: 1,
            }
        );
        assert_eq!(
            Meta::index_blocks(BLOCK_SIZE - 1),
            LevelInfo {
                l4: 0,
                l3: 0,
                block_directory: 0,
                block_table: 0,
                direct: 1,
            }
        );
        assert_eq!(
            Meta::index_blocks(BLOCK_SIZE),
            LevelInfo {
                l4: 0,
                l3: 0,
                block_directory: 0,
                block_table: 0,
                direct: 1,
            }
        );
        assert_eq!(
            Meta::index_blocks(BLOCK_SIZE + 1),
            LevelInfo {
                l4: 0,
                l3: 0,
                block_directory: 0,
                block_table: 0,
                direct: 2,
            }
        );
        assert_eq!(
            Meta::index_blocks(DIRECT_MAX - 10),
            LevelInfo {
                l4: 0,
                l3: 0,
                block_directory: 0,
                block_table: 0,
                direct: DIRECT_COUNT as u64,
            }
        );
        assert_eq!(
            Meta::index_blocks(DIRECT_MAX),
            LevelInfo {
                l4: 0,
                l3: 0,
                block_directory: 0,
                block_table: 0,
                direct: DIRECT_COUNT as u64,
            }
        );
        assert_eq!(
            Meta::index_blocks(DIRECT_MAX + 1),
            LevelInfo {
                l4: 0,
                l3: 0,
                block_directory: 0,
                block_table: 1,
                direct: DIRECT_COUNT as u64,
            }
        );
        let size = BLOCK_TABLE_MAX;
        assert_eq!(
            Meta::index_blocks(size),
            LevelInfo {
                l4: 0,
                l3: 0,
                block_directory: 0,
                block_table: 1,
                direct: DIRECT_COUNT as u64,
            }
        );
        let size = BLOCK_TABLE_MAX + 1;
        assert_eq!(
            Meta::index_blocks(size),
            LevelInfo {
                l4: 0,
                l3: 0,
                block_directory: 1,
                block_table: 2,
                direct: DIRECT_COUNT as u64,
            }
        );
        let size = L3_MAX;
        assert_eq!(
            Meta::index_blocks(size),
            LevelInfo {
                l4: 0,
                l3: 1,
                block_directory: (size - DIRECT_MAX + BLOCK_DIRECTORY_INDIRECT_MAX - 1)
                    / BLOCK_DIRECTORY_INDIRECT_MAX,
                block_table: (size - DIRECT_MAX + BLOCK_TABLE_INDIRECT_MAX - 1)
                    / BLOCK_TABLE_INDIRECT_MAX,
                direct: DIRECT_COUNT as u64,
            }
        );
        let size = L3_MAX + 1;
        assert_eq!(
            Meta::index_blocks(size),
            LevelInfo {
                l4: 1,
                l3: 2,
                block_directory: (size - DIRECT_MAX + BLOCK_DIRECTORY_INDIRECT_MAX - 1)
                    / BLOCK_DIRECTORY_INDIRECT_MAX,
                block_table: (size - DIRECT_MAX + BLOCK_TABLE_INDIRECT_MAX - 1)
                    / BLOCK_TABLE_INDIRECT_MAX,
                direct: DIRECT_COUNT as u64,
            }
        );
    }

    #[test]
    fn test_to_vec_filter() {
        let (inode, index_ids, block_ids, cache_manager, _) = fake_inode(2 * BLOCK_DIRECTORY_MAX);
        unsafe {
            let block_directory_id = cache_manager
                .get(inode.indirect)
                .read()
                .get_ref::<IndirectBlock>(0)
                .entries[1];
            let (filtered_index, filtered_data) = cache_manager
                .get(inode.indirect)
                .read()
                .get_ref::<IndirectBlock>(0)
                .to_vec(cache_manager.clone(), Some(&vec![block_directory_id]));
            assert_eq!(
                inode.data_blocks(),
                (filtered_data.len() + DIRECT_COUNT + INDIRECT_LEN * INDIRECT_LEN) as u64
            );
            assert_eq!(
                Meta::index_blocks(inode.size).index_block_count(),
                1 + filtered_index.len() as u64 + 64
            );
        }
    }

    #[test]
    fn test_shrink() {
        let mut prev_size = 2 * BLOCK_DIRECTORY_MAX;
        // let mut prev_size = DIRECT_MAX + BLOCK_SIZE;
        // let shrink_size = [DIRECT_MAX];
        let shrink_size = [
            BLOCK_DIRECTORY_MAX,
            BLOCK_DIRECTORY_MAX - 10,
            BLOCK_DIRECTORY_MAX - BLOCK_SIZE,
            10 * BLOCK_TABLE_MAX,
            3 * BLOCK_TABLE_MAX,
            BLOCK_TABLE_MAX,
            BLOCK_TABLE_MAX - BLOCK_SIZE + 10,
            BLOCK_TABLE_MAX - BLOCK_SIZE,
            BLOCK_TABLE_MAX - BLOCK_SIZE - 10,
            BLOCK_TABLE_MAX - 2 * BLOCK_SIZE,
            2 * DIRECT_MAX + 10,
            DIRECT_MAX + 10 * BLOCK_SIZE + 20,
            DIRECT_MAX + BLOCK_SIZE,
            DIRECT_MAX,
            10 * BLOCK_SIZE,
            10 * BLOCK_SIZE,
            5 * BLOCK_SIZE - 10,
            3 * BLOCK_SIZE + 200,
            BLOCK_SIZE,
            300,
            0,
        ];
        let (mut inode, index_ids, block_ids, cache_manager, _) = fake_inode(prev_size);

        for new_size in shrink_size {
            println!("{} {}", prev_size, new_size);
            let prev_info = Meta::index_blocks(prev_size);
            let (index_ids, block_ids) = inode.blocks(cache_manager.clone());
            assert_eq!(prev_info.index_block_count(), index_ids.len() as u64);

            let prev_index_blocks = prev_info.index_block_count();
            let prev_data_blocks = inode.data_blocks();

            let (dealloc_index_ids, dealloc_data_ids) =
                inode.shrink(new_size, cache_manager.clone());
            let new_info = Meta::index_blocks(inode.size);
            let new_index_blocks = new_info.index_block_count();
            let new_data_blocks = inode.data_blocks();

            let (index, blocks) = inode.blocks(cache_manager.clone());
            assert_eq!(index.len() as u64, new_info.index_block_count());
            assert_eq!(blocks.len() as u64, new_data_blocks);
            assert_eq!(
                prev_index_blocks,
                dealloc_index_ids.len() as u64 + new_index_blocks
            );
            assert_eq!(
                prev_data_blocks,
                dealloc_data_ids.len() as u64 + new_data_blocks
            );
            prev_size = new_size
        }
    }

    #[test]
    fn test_expand() {
        let (mut inode, index_ids, block_ids, cache_manager, mut id_iter) = fake_inode(200);
        // let extend_size = [BLOCK_TABLE_MAX * 2];
        let extend_size = [
            // direct
            300,
            BLOCK_SIZE,
            BLOCK_SIZE + 20,
            BLOCK_SIZE * 2,
            DIRECT_MAX - 50,
            DIRECT_MAX,
            // indirect
            DIRECT_MAX + 30,
            DIRECT_MAX + BLOCK_SIZE,
            DIRECT_MAX + BLOCK_SIZE + 10,
            DIRECT_MAX + BLOCK_SIZE * 2,
            DIRECT_MAX * 2,
            BLOCK_TABLE_MAX,
            BLOCK_TABLE_MAX + 10,
            BLOCK_TABLE_MAX + DIRECT_MAX,
            BLOCK_TABLE_MAX + BLOCK_TABLE_INDIRECT_MAX,
            BLOCK_TABLE_MAX * 2,
            BLOCK_TABLE_MAX * 2 + DIRECT_MAX,
            BLOCK_DIRECTORY_MAX,
            BLOCK_DIRECTORY_MAX + DIRECT_MAX - 20,
            L3_MAX,
            L3_MAX + BLOCK_DIRECTORY_MAX,
            L3_MAX * 2,
        ];

        for new_size in extend_size {
            let curr_info = Meta::index_blocks(inode.size());

            let new_info = Meta::index_blocks(new_size);
            let new_index_blocks = new_info.index_block_count();
            let data_blocks = id_iter
                .by_ref()
                .take((Meta::_data_blocks(new_size) - inode.data_blocks()) as usize)
                .collect::<Vec<_>>();
            let index_blocks = id_iter
                .by_ref()
                .take((new_info.index_block_count() - curr_info.index_block_count()) as usize)
                .collect::<Vec<_>>();
            inode.extend(
                new_size,
                new_info,
                data_blocks,
                index_blocks,
                cache_manager.clone(),
            );

            let (index_ids, block_ids) = inode.blocks(cache_manager.clone());
            assert_eq!(new_index_blocks, index_ids.len() as u64);
            assert_eq!(inode.data_blocks(), block_ids.len() as u64);
        }
    }

    #[test]
    fn test_clear_size() {
        let (mut inode, index_ids, block_ids, cache_manager, _) = fake_inode(BLOCK_TABLE_MAX * 2);

        let ids = inode.clear_size(cache_manager);

        assert_eq!(ids.len(), index_ids.len() + block_ids.len())
    }
}
