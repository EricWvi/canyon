use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::RwLock;

pub trait FS {
    fn create(&self, parent: u64, name: String) -> Arc<RwLock<dyn Inode>>;
    fn write(&self, inode_number: u64, contents: &Vec<u8>);
    fn df(&self) -> (u64, u64);

    fn inode(&self, inode_number: u64) -> Arc<RwLock<dyn Inode>>;
    fn sub_inodes(&self, inode_number: u64) -> Vec<u64>;
}

#[derive(PartialEq, Copy, Clone)]
#[repr(u64)]
pub enum InodeType {
    File,
    Dir,
}

pub trait Inode {
    fn inode_number(&self) -> u64;
    fn inode_type(&self) -> InodeType;
    fn is_file(&self) -> bool;
    fn data(&self) -> Vec<u8>;
    fn name(&self) -> String;
    fn size(&self) -> u64;
}
