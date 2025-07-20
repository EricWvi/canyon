#![no_std]

extern crate alloc;

use alloc::vec::Vec;
use uefi::proto::console::gop::ModeInfo;
use uefi::table::boot::MemoryDescriptor;

/// information that bootloader passes to the kernel
#[repr(C)]
pub struct BootInfo {
    pub memory_map: Vec<&'static MemoryDescriptor>,
    /// The offset where the physical memory is mapped at in the virtual address space.
    pub physical_memory_offset: u64,
    /// The graphic output information
    pub graphic_info: GraphicInfo,
}

/// Graphic output information
#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct GraphicInfo {
    /// Graphic mode
    pub mode: ModeInfo,
    /// Framebuffer base physical address
    pub fb_addr: u64,
    /// Framebuffer size
    pub fb_size: u64,
}
