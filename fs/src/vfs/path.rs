use crate::vfs::VFS;
use alloc::string::String;
use alloc::vec::Vec;

pub struct Path {
    pub(crate) parents: Vec<String>,
    pub(crate) name: String,
}

impl VFS {
    #[inline]
    pub(crate) fn parse_path(path: &str) -> Path {
        let mut dirs = path
            .split('/')
            .skip(1)
            .map(|s| String::from(s))
            .collect::<Vec<_>>();
        let name = dirs.pop().unwrap();
        Path {
            parents: dirs,
            name,
        }
    }
}
