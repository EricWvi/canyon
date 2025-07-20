use crate::fs::{InodeType, FS};
use crate::vfs::VFS;
use alloc::string::String;
use alloc::sync::{Arc, Weak};
use alloc::vec;
use alloc::vec::Vec;
use spin::RwLock;

pub struct DirEntry {
    pub(crate) inode_type: InodeType,
    pub(crate) inode_number: u64,
    pub(crate) fs: Weak<dyn FS>,
    pub(crate) name: String,
    pub(crate) parent: Option<Weak<RwLock<DirEntry>>>,
    pub(crate) subdirs: Vec<Arc<RwLock<DirEntry>>>,
}

impl DirEntry {
    pub fn new(
        parent: Option<Weak<RwLock<DirEntry>>>,
        inode_number: u64,
        fs: Weak<dyn FS>,
    ) -> Self {
        let inode = fs.upgrade().unwrap().inode(inode_number);
        let name = inode.read().name();
        let inode_type = inode.read().inode_type();
        Self {
            parent,
            name,
            inode_number,
            inode_type,
            fs,
            subdirs: vec![],
        }
    }

    pub fn inode_number(&self) -> u64 {
        self.inode_number
    }

    pub fn read_sub_dentry(parent_dentry: Arc<RwLock<DirEntry>>) {
        let fs = parent_dentry.read().fs.upgrade().unwrap();
        let sub_inodes = fs.sub_inodes(parent_dentry.read().inode_number);

        for inode_number in sub_inodes {
            let dentry = Arc::new(RwLock::new(DirEntry::new(
                Some(Arc::downgrade(&parent_dentry)),
                inode_number,
                parent_dentry.read().fs.clone(),
            )));
            Self::read_sub_dentry(dentry.clone());

            parent_dentry.write().subdirs.push(dentry);
        }
    }
}

impl VFS {
    pub(crate) fn find_dentry(
        &self,
        dirs: &Vec<String>,
    ) -> Result<Arc<RwLock<DirEntry>>, crate::Error> {
        let mut dir = self.dentry_cache.clone();
        for parent_dir in dirs {
            let next = if let Some(d) = dir.read().subdirs.iter().find(|d| {
                let d = d.read();
                d.name == *parent_dir
            }) {
                d.clone()
            } else {
                return Err(crate::Error::NotExist(parent_dir.clone()));
            };
            dir = next;
        }
        Ok(dir)
    }
}
