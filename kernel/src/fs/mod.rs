use alloc::sync::Arc;
use cafs::vfs::VFS;

pub static mut VFS: Option<Arc<VFS>> = None;
