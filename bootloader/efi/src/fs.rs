use alloc::vec;
use log::info;
use uefi::prelude::BootServices;
use uefi::proto::media::file::{File, FileAttribute, FileInfo, FileMode, RegularFile};
use uefi::proto::media::fs::SimpleFileSystem;
use uefi::table::boot::{AllocateType, MemoryType};
use uefi::CStr16;

pub fn open_file(bs: &BootServices, path: &str) -> RegularFile {
    info!("opening file: {}", path);
    let fs = unsafe {
        bs.locate_protocol::<SimpleFileSystem>()
            .expect("failed to get SimpleFileSystem")
    };
    let fs = unsafe { &mut *fs.get() };
    let mut root = match fs.open_volume() {
        Err(e) => panic!("{:?}", e),
        Ok(dir) => dir,
    };
    let mut buf = vec![0; path.len() + 1];
    let path_cstr = match CStr16::from_str_with_buf(path, &mut buf) {
        Err(e) => panic!("{:?}", e),
        Ok(str) => str,
    };
    match root.open(path_cstr, FileMode::Read, FileAttribute::empty()) {
        Ok(handle) => unsafe { RegularFile::new(handle) },
        Err(e) => panic!("{:?}", e),
    }
}

/// Load file to new allocated pages
pub fn load_file(bs: &BootServices, file: &mut RegularFile) -> &'static mut [u8] {
    info!("loading file to memory");
    let mut info_buf = vec![0u8; 0x100];
    let info = file
        .get_info::<FileInfo>(&mut info_buf)
        .expect("failed to get file info");
    let pages = info.file_size() as usize / 0x1000 + 1;
    let mem_start = bs
        .allocate_pages(AllocateType::AnyPages, MemoryType::LOADER_DATA, pages)
        .expect("failed to allocate pages");
    let buf = unsafe { core::slice::from_raw_parts_mut(mem_start as *mut u8, pages * 0x1000) };
    let len = file.read(buf).expect("failed to read file");
    &mut buf[..len]
}
