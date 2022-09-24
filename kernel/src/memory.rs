use alloc::vec::Vec;
use uefi::table::boot::MemoryDescriptor;
use x86_64::{PhysAddr, VirtAddr};

static mut PHYSICAL_MEMORY_OFFSET: Option<u64> = None;

pub fn init(offset: u64, descriptors: &Vec<&MemoryDescriptor>) {
    unsafe {
        PHYSICAL_MEMORY_OFFSET = Some(offset);
    }
}

pub fn to_virt_addr(phys: u64) -> VirtAddr {
    unsafe { VirtAddr::new(phys + PHYSICAL_MEMORY_OFFSET.unwrap()) }
}
