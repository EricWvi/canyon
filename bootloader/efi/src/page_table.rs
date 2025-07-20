use log::{debug, info, trace};
use uefi::prelude::BootServices;
use uefi::table::boot::{AllocateType, MemoryType};
use x86_64::registers::control::Cr3;
use x86_64::structures::paging::mapper::{MapToError, UnmapError};
use x86_64::structures::paging::*;
use x86_64::{align_up, PhysAddr, VirtAddr};
use xmas_elf::program;
use xmas_elf::ElfFile;

/// Use `BootServices::allocate_pages()` as frame allocator
pub struct UEFIFrameAllocator<'a>(pub &'a BootServices);

unsafe impl FrameAllocator<Size4KiB> for UEFIFrameAllocator<'_> {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        let addr = self
            .0
            .allocate_pages(AllocateType::AnyPages, MemoryType::LOADER_DATA, 1)
            .expect("failed to allocate frame");
        let frame = PhysFrame::containing_address(PhysAddr::new(addr));
        Some(frame)
    }
}

pub fn p4_table() -> OffsetPageTable<'static> {
    let p4_table_addr = Cr3::read().0.start_address().as_u64();
    let p4_table = unsafe { &mut *(p4_table_addr as *mut PageTable) };
    // NOTE
    // During BootServices, UEFI and bootloader runs identity mapped.
    unsafe { OffsetPageTable::new(p4_table, VirtAddr::new(0)) }
}

pub fn map_elf(
    elf: &ElfFile,
    page_table: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> Result<(), MapToError<Size4KiB>> {
    info!("Mapping ELF file.");
    let kernel_start = PhysAddr::new(elf.input.as_ptr() as u64);
    for segment in elf.program_iter() {
        map_segment(&segment, kernel_start, page_table, frame_allocator)?;
    }
    Ok(())
}

fn map_segment(
    segment: &program::ProgramHeader,
    kernel_start: PhysAddr,
    page_table: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> Result<(), MapToError<Size4KiB>> {
    if segment.get_type().unwrap() != program::Type::Load {
        return Ok(());
    }
    debug!("Mapping segment: {:#x?}", segment);

    let mem_size = segment.mem_size(); // Size in bytes of the segment in memory
    let file_size = segment.file_size(); // Size in bytes of the segment in the file image
    let file_offset = segment.offset() & !0xfff; // Offset of the segment in the file image
                                                 // 4K aligned
    let phys_start_addr = kernel_start + file_offset;
    let virt_start_addr = VirtAddr::new(segment.virtual_addr());

    let start_page = Page::containing_address(virt_start_addr);
    let start_frame = PhysFrame::containing_address(phys_start_addr);
    let end_frame = PhysFrame::containing_address(phys_start_addr + file_size - 1u64);

    let flags = segment.flags();
    let mut page_table_flags = PageTableFlags::PRESENT;
    if !flags.is_execute() {
        page_table_flags |= PageTableFlags::NO_EXECUTE
    };
    if flags.is_write() {
        page_table_flags |= PageTableFlags::WRITABLE
    };

    for frame in PhysFrame::range_inclusive(start_frame, end_frame) {
        let offset = frame - start_frame;
        let page = start_page + offset;
        unsafe {
            page_table
                .map_to(page, frame, page_table_flags, frame_allocator)?
                .flush();
        }
    }

    if mem_size > file_size {
        // .bss section (or similar), which needs to be zeroed
        let zero_start = virt_start_addr + file_size;
        let zero_end = virt_start_addr + mem_size;
        if zero_start.as_u64() & 0xfff != 0 {
            // A part of the last mapped frame needs to be zeroed. This is
            // not possible since it could already contains parts of the next
            // segment. Thus, we need to copy it before zeroing.
            // i.e. maybe there is another page point to this frame

            let new_frame = frame_allocator
                .allocate_frame()
                .ok_or(MapToError::FrameAllocationFailed)?;

            type PageArray = [u64; Size4KiB::SIZE as usize / 8];

            let last_page = Page::containing_address(virt_start_addr + file_size - 1u64);
            let last_page_ptr = end_frame.start_address().as_u64() as *mut PageArray;
            let temp_page_ptr = new_frame.start_address().as_u64() as *mut PageArray;

            unsafe {
                // copy contents
                temp_page_ptr.write(last_page_ptr.read());
            }

            // remap last page
            if let Err(e) = page_table.unmap(last_page.clone()) {
                return Err(match e {
                    UnmapError::ParentEntryHugePage => MapToError::ParentEntryHugePage,
                    UnmapError::PageNotMapped => unreachable!(),
                    UnmapError::InvalidFrameAddress(_) => unreachable!(),
                });
            }
            unsafe {
                page_table
                    .map_to(last_page, new_frame, page_table_flags, frame_allocator)?
                    .flush();
            }
        }

        // Map additional frames.
        let start_page: Page =
            Page::containing_address(VirtAddr::new(align_up(zero_start.as_u64(), Size4KiB::SIZE)));
        let end_page = Page::containing_address(zero_end);
        for page in Page::range_inclusive(start_page, end_page) {
            let frame = frame_allocator
                .allocate_frame()
                .ok_or(MapToError::FrameAllocationFailed)?;
            unsafe {
                page_table
                    .map_to(page, frame, page_table_flags, frame_allocator)?
                    .flush();
            }
        }

        // zero bss
        unsafe {
            core::ptr::write_bytes(
                zero_start.as_mut_ptr::<u8>(),
                0,
                (mem_size - file_size) as usize,
            );
        }
    }

    Ok(())
}

pub fn map_stack(
    addr: u64,
    pages: u64,
    page_table: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> Result<(), MapToError<Size4KiB>> {
    info!("Mapping stack at {:#x}.", addr);
    let stack_start = Page::containing_address(VirtAddr::new(addr));
    let stack_end = stack_start + pages;
    debug!("stack_start: {:#x}", stack_start.start_address().as_u64());
    debug!("stack_end: {:#x}", stack_end.start_address().as_u64() - 1);

    let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;

    for page in Page::range(stack_start, stack_end) {
        let frame = frame_allocator
            .allocate_frame()
            .ok_or(MapToError::FrameAllocationFailed)?;
        unsafe {
            page_table
                .map_to(page, frame, flags, frame_allocator)?
                .flush();
        }
    }

    Ok(())
}

pub fn map_physical_memory(
    offset: u64,
    max_addr: u64,
    page_table: &mut impl Mapper<Size2MiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) {
    info!("mapping physical memory");
    let start_frame = PhysFrame::containing_address(PhysAddr::new(0));
    let end_frame = PhysFrame::containing_address(PhysAddr::new(max_addr));
    for frame in PhysFrame::range_inclusive(start_frame, end_frame) {
        let page = Page::containing_address(VirtAddr::new(frame.start_address().as_u64() + offset));
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
        unsafe {
            page_table
                .map_to(page, frame, flags, frame_allocator)
                .expect("failed to map physical memory")
                .flush();
        }
    }
}
