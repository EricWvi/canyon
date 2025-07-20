#![no_main]
#![no_std]
#![feature(abi_efiapi)]

mod config;
mod fs;
mod page_table;

extern crate alloc;

use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec;
use bootloader_lib::*;
use core::arch::asm;
use core::cmp::max;
use log::{debug, info};
use uefi::prelude::*;
use uefi::proto::console::gop::GraphicsOutput;
use x86_64::registers::control::{Cr0, Cr0Flags, Efer, EferFlags};
use xmas_elf::ElfFile;

static mut ENTRY: usize = 0;

#[entry]
fn main(handle: Handle, mut system_table: SystemTable<Boot>) -> Status {
    uefi_services::init(&mut system_table).unwrap();
    info!("bootloader starts working...");
    let bs = system_table.boot_services();

    // Get memory map.
    // From uefi-rs:
    // Note that the size of the memory map can increase any time an allocation happens,
    // so when creating a buffer to put the memory map into, it's recommended to allocate a few extra
    // elements worth of space above the size of the current memory map.
    let mmap_storage = Box::leak(vec![0; bs.memory_map_size().map_size * 2].into_boxed_slice());
    let mmap_iter = bs
        .memory_map(mmap_storage)
        .expect("failed to get memory map iter")
        .1;
    let mmap_len = mmap_iter.len();
    let max_phys_addr = mmap_iter
        .map(|x| x.phys_start + x.page_count * 0x1000)
        .max()
        .unwrap();

    // Read config.
    let config = {
        let mut conf = fs::open_file(bs, "\\EFI\\Boot\\boot.conf");
        let buf = fs::load_file(bs, &mut conf);
        config::Config::parse(buf)
    };

    let graphic_info = init_graphic(bs, config.resolution);
    debug!("graphic_info {:#?}", graphic_info);

    // Read kernel.
    let elf = {
        let mut file = fs::open_file(bs, config.kernel_path);
        let buf = fs::load_file(bs, &mut file);
        ElfFile::new(buf).expect("failed to parse ELF")
    };

    unsafe {
        ENTRY = elf.header.pt2.entry_point() as usize;
    }

    // Map virtual memory.
    unsafe {
        // remove write protect
        Cr0::update(|f| f.remove(Cr0Flags::WRITE_PROTECT));
        // enable protection against malicious code from non-executable memory locations
        Efer::update(|f| f.insert(EferFlags::NO_EXECUTE_ENABLE));
    }
    let mut page_table = page_table::p4_table();
    let mut frame_allocator = page_table::UEFIFrameAllocator(bs);
    // Map kernel.
    page_table::map_elf(&elf, &mut page_table, &mut frame_allocator).expect("failed to map ELF");
    // Map stack.
    page_table::map_stack(
        config.kernel_stack_address,
        config.kernel_stack_size,
        &mut page_table,
        &mut frame_allocator,
    )
    .expect("failed to map stack");
    // Map physical memory.
    page_table::map_physical_memory(
        config.physical_memory_offset,
        max_phys_addr,
        &mut page_table,
        &mut frame_allocator,
    );
    unsafe {
        // recover write protect
        Cr0::update(|f| f.insert(Cr0Flags::WRITE_PROTECT));
    }

    let stacktop = config.kernel_stack_address + config.kernel_stack_size * 0x1000;
    info!("stacktop at {:#x}", stacktop);

    info!("exiting boot services");

    let mut memory_map = Vec::with_capacity(mmap_len * 2);

    let (_rs, mut mmap_iter) = system_table
        .exit_boot_services(handle, mmap_storage)
        .expect("Failed to exit boot services");

    // ---------------------------------------------
    // !! NOTE: alloc & log can no longer be used
    // ---------------------------------------------

    for desc in mmap_iter {
        memory_map.push(desc);
    }

    let boot_info = BootInfo {
        memory_map: memory_map,
        physical_memory_offset: config.physical_memory_offset,
        graphic_info,
    };

    unsafe {
        jump_to_entry(stacktop, &boot_info);
    }
}

unsafe fn jump_to_entry(stack_top: u64, boot_info: *const BootInfo) -> ! {
    asm!("mov rsp, {}; call {}", in(reg) stack_top, in(reg)ENTRY, in("rdi")boot_info);
    loop {
        asm!("nop");
    }
}

/// If `resolution` is some, then set graphic mode matching the resolution.
/// Return information of the final graphic mode.
fn init_graphic(bs: &BootServices, resolution: Option<(usize, usize)>) -> GraphicInfo {
    let gop = unsafe {
        bs.locate_protocol::<GraphicsOutput>()
            .expect("failed to get GraphicsOutput")
    };
    let gop = unsafe { &mut *gop.get() };

    if let Some(resolution) = resolution {
        let mode = gop
            .modes()
            .find(|mode| {
                let info = mode.info();
                info.resolution() == resolution
            })
            .expect("graphic mode not found");
        info!("switching graphic mode");
        gop.set_mode(&mode).expect("Failed to set graphics mode");
    }
    GraphicInfo {
        mode: gop.current_mode_info(),
        fb_addr: gop.frame_buffer().as_mut_ptr() as u64,
        fb_size: gop.frame_buffer().size() as u64,
    }
}
