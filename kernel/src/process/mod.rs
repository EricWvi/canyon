mod context;
pub mod elf;
mod thread;

use crate::fs;
use alloc::string::String;
use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;
use core::cell::RefCell;
use core::mem;
use log::info;
use spin::Mutex;
use xmas_elf::header::HeaderPt2_;
use xmas_elf::{ElfFile, P64};

static mut PROCESS_COUNT: Mutex<u64> = Mutex::new(0);

pub fn next() -> u64 {
    unsafe {
        let mut pc = PROCESS_COUNT.lock();
        *pc += 1;
        *pc - 1
    }
}

pub fn init() {
    let contents = unsafe { fs::VFS.as_ref().unwrap().read_unstable("/hello").unwrap() };
    info!("hello len: {}", contents.len());
    assert!((18446620929482082386 & (mem::align_of::<HeaderPt2_<P64>>() - 1)) == 0);
    // let elf = ElfFile::new(contents.as_slice()).expect("failed to parse ELF");
    // elf::map_elf(&elf);
}

pub struct Process {
    pid: usize,
    name: String,
    page_table: u64,
}

pub struct Stack {
    pid: usize,
}

// pub struct PCB {
//     pub pid: u64,
//     pub stack: Stack,
//     inner: RefCell<PCBInner>,
// }

// pub struct PCBInner {
//     pub trap_ctx_ppn: PhysPageNum,
//     pub base_size: usize,
//     pub ctx: ProcessContext,
//     pub status: ProcessStatus,
//     pub memory_set: MemorySet,
//     pub parent: Option<Weak<PCB>>,
//     pub children: Vec<Arc<PCB>>,
//     pub exit_code: i32,
// }

struct ProcessList {
    ready: Vec<Arc<Process>>,
}
