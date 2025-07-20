pub mod frame;
pub mod page;

use alloc::vec::Vec;
use frame::MemoryRange;
use uefi::table::boot::{MemoryDescriptor, MemoryType};
use x86_64::registers::control::Cr3;
use x86_64::structures::paging::PhysFrame;
use x86_64::{PhysAddr, VirtAddr};
use crate::memory::frame::init_frame;

static mut PHYSICAL_MEMORY_OFFSET: u64 = 0;
static mut KERNEL_P4_TABLE: u64 = 0;

pub const PAGE_SIZE: usize = 1 << 12;

pub fn init(offset: u64, descriptors: &Vec<&MemoryDescriptor>) {
    unsafe {
        PHYSICAL_MEMORY_OFFSET = offset;
        KERNEL_P4_TABLE = Cr3::read().0.start_address().as_u64();
        init_frame(descriptors
            .iter()
            .filter(|x| x.ty == MemoryType::CONVENTIONAL)
            .map(|x| MemoryRange {
                start: PhysFrame::containing_address(PhysAddr::new(x.phys_start)),
                pages: x.page_count,
            })
            .collect::<Vec<MemoryRange>>());
    }
}

pub fn to_virt_addr(phys: u64) -> VirtAddr {
    unsafe { VirtAddr::new(phys + PHYSICAL_MEMORY_OFFSET) }
}

pub fn to_phys_addr(virt: u64) -> PhysAddr {
    unsafe { PhysAddr::new(virt - PHYSICAL_MEMORY_OFFSET) }
}
