use log::info;
use x86_64::structures::paging::mapper::{MapToError, UnmapError};
use x86_64::structures::paging::*;
use x86_64::PhysAddr;
use xmas_elf::program;
use xmas_elf::ElfFile;

pub fn map_elf(
    elf: &ElfFile,
    // page_table: &mut impl Mapper<Size4KiB>,
    // frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> Result<(), MapToError<Size4KiB>> {
    let user_start = PhysAddr::new(elf.input.as_ptr() as u64);
    info!("{:?}", user_start);
    for segment in elf.program_iter() {
        // map_segment(&segment, user_start, page_table, frame_allocator)?;
    }
    Ok(())
}

// fn map_segment(
//     segment: &program::ProgramHeader,
//     kernel_start: PhysAddr,
//     page_table: &mut impl Mapper<Size4KiB>,
//     frame_allocator: &mut impl FrameAllocator<Size4KiB>,
// ) -> Result<(), MapToError<Size4KiB>> {
//     if segment.get_type().unwrap() != program::Type::Load {
//         return Ok(());
//     }
//     debug!("Mapping segment: {:#x?}", segment);
//
//     let mem_size = segment.mem_size(); // Size in bytes of the segment in memory
//     let file_size = segment.file_size(); // Size in bytes of the segment in the file image
//     let file_offset = segment.offset() & !0xfff; // Offset of the segment in the file image
//                                                  // 4K aligned
//     let phys_start_addr = kernel_start + file_offset;
//     let virt_start_addr = VirtAddr::new(segment.virtual_addr());
//
//     let start_page = Page::containing_address(virt_start_addr);
//     let start_frame = PhysFrame::containing_address(phys_start_addr);
//     let end_frame = PhysFrame::containing_address(phys_start_addr + file_size - 1u64);
//
//     let flags = segment.flags();
//     let mut page_table_flags = PageTableFlags::PRESENT;
//     if !flags.is_execute() {
//         page_table_flags |= PageTableFlags::NO_EXECUTE
//     };
//     if flags.is_write() {
//         page_table_flags |= PageTableFlags::WRITABLE
//     };
//
//     for frame in PhysFrame::range_inclusive(start_frame, end_frame) {
//         let offset = frame - start_frame;
//         let page = start_page + offset;
//         unsafe {
//             page_table
//                 .map_to(page, frame, page_table_flags, frame_allocator)?
//                 .flush();
//         }
//     }
//
//     if mem_size > file_size {
//         // .bss section (or similar), which needs to be zeroed
//         let zero_start = virt_start_addr + file_size;
//         let zero_end = virt_start_addr + mem_size;
//         if zero_start.as_u64() & 0xfff != 0 {
//             // A part of the last mapped frame needs to be zeroed. This is
//             // not possible since it could already contains parts of the next
//             // segment. Thus, we need to copy it before zeroing.
//             // i.e. maybe there is another page point to this frame
//
//             let new_frame = frame_allocator
//                 .allocate_frame()
//                 .ok_or(MapToError::FrameAllocationFailed)?;
//
//             type PageArray = [u64; Size4KiB::SIZE as usize / 8];
//
//             let last_page = Page::containing_address(virt_start_addr + file_size - 1u64);
//             let last_page_ptr = end_frame.start_address().as_u64() as *mut PageArray;
//             let temp_page_ptr = new_frame.start_address().as_u64() as *mut PageArray;
//
//             unsafe {
//                 // copy contents
//                 temp_page_ptr.write(last_page_ptr.read());
//             }
//
//             // remap last page
//             if let Err(e) = page_table.unmap(last_page.clone()) {
//                 return Err(match e {
//                     UnmapError::ParentEntryHugePage => MapToError::ParentEntryHugePage,
//                     UnmapError::PageNotMapped => unreachable!(),
//                     UnmapError::InvalidFrameAddress(_) => unreachable!(),
//                 });
//             }
//             unsafe {
//                 page_table
//                     .map_to(last_page, new_frame, page_table_flags, frame_allocator)?
//                     .flush();
//             }
//         }
//
//         // Map additional frames.
//         let start_page: Page =
//             Page::containing_address(VirtAddr::new(align_up(zero_start.as_u64(), Size4KiB::SIZE)));
//         let end_page = Page::containing_address(zero_end);
//         for page in Page::range_inclusive(start_page, end_page) {
//             let frame = frame_allocator
//                 .allocate_frame()
//                 .ok_or(MapToError::FrameAllocationFailed)?;
//             unsafe {
//                 page_table
//                     .map_to(page, frame, page_table_flags, frame_allocator)?
//                     .flush();
//             }
//         }
//
//         // zero bss
//         unsafe {
//             core::ptr::write_bytes(
//                 zero_start.as_mut_ptr::<u8>(),
//                 0,
//                 (mem_size - file_size) as usize,
//             );
//         }
//     }
//
//     Ok(())
// }
