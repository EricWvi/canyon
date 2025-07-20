mod dir_entry;
mod path;

use crate::cafs::CAFS;
use crate::fs::FS;
use crate::BlockDevice;
use alloc::string::String;
use alloc::sync::{Arc, Weak};
use alloc::vec;
use alloc::vec::Vec;
use log::info;
use spin::RwLock;

pub use dir_entry::*;
pub use path::*;

pub struct VFS {
    primary_partition: Arc<dyn FS>,
    dentry_cache: Arc<RwLock<DirEntry>>,
}

impl VFS {
    pub fn new(block_device: Arc<RwLock<dyn BlockDevice>>) -> Arc<VFS> {
        let fs: Arc<dyn FS> = Arc::new(CAFS::open(block_device));
        let root_dentry = Arc::new(RwLock::new(DirEntry::new(None, 0, Arc::downgrade(&fs))));
        DirEntry::read_sub_dentry(root_dentry.clone());
        Arc::new(Self {
            primary_partition: fs,
            dentry_cache: root_dentry,
        })
    }

    pub fn ls_root(&self) -> Vec<String> {
        let root = self.dentry_cache.read();
        let mut strs = vec![root.name.clone()];
        for dentry in &root.subdirs {
            let d = dentry.read();
            strs.push(d.name.clone());
        }
        strs
    }
}

impl VFS {
    pub fn create(&self, path: &str) -> Result<(), crate::Error> {
        let Path { name, parents } = Self::parse_path(path);
        let dir = match self.find_dentry(&parents) {
            Ok(dir) => dir,
            Err(e) => return Err(e),
        };
        // create inode
        let inode_meta = self
            .primary_partition
            .create(dir.read().inode_number(), name);
        {
            // add to dentry cache
            let parent = Arc::downgrade(&dir);
            let mut dir = dir.write();
            let fs = dir.fs.clone();
            dir.subdirs.push(Arc::new(RwLock::new(DirEntry::new(
                Some(parent),
                inode_meta.read().inode_number(),
                fs,
            ))));
        }
        Ok(())
    }

    pub fn open() {}

    pub fn read_unstable(&self, path: &str) -> Result<Vec<u8>, crate::Error> {
        let p = Self::parse_path(path);
        let mut path = p.parents;
        path.push(p.name);

        let dentry = match self.find_dentry(&path) {
            Ok(d) => d,
            Err(e) => return Err(e),
        };
        let number = dentry.read().inode_number();
        let fs = dentry.read().fs.upgrade().unwrap();
        Ok(fs.inode(number).read().data())
    }

    // TODO refactor write and create
    pub fn write(&self, path: &str, contents: &Vec<u8>) -> Result<(), crate::Error> {
        let p = Self::parse_path(path);
        let mut path = p.parents;
        path.push(p.name);

        let dentry = match self.find_dentry(&path) {
            Ok(d) => d,
            Err(e) => return Err(e),
        };
        self.primary_partition
            .write(dentry.read().inode_number(), contents);
        Ok(())
    }
}
