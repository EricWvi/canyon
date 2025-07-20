use alloc::vec::Vec;
use x86_64::instructions::segmentation::{Segment, CS, SS};
use x86_64::instructions::tables::load_tss;
use x86_64::structures::gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector};
use x86_64::structures::tss::TaskStateSegment;
use x86_64::VirtAddr;

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

static mut GDT: Option<GlobalDescriptorTable> = None;
static mut TSS: Option<TaskStateSegment> = None;

pub fn init() {
    // To prevent triple faults in all cases, we also set up an Interrupt Stack Table
    // to catch double faults on a separate kernel stack.
    let tss = unsafe {
        TSS = Some(TaskStateSegment::new());
        TSS.as_mut().unwrap()
    };
    tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
        let stack = Vec::<u8>::with_capacity(4096 * 5).leak();
        let stack_start = VirtAddr::from_ptr(stack.as_ptr());
        let stack_end = stack_start + stack.len();
        stack_end
    };
    let gdt = unsafe {
        GDT = Some(GlobalDescriptorTable::new());
        GDT.as_mut().unwrap()
    };
    let code_selector = gdt.add_entry(Descriptor::kernel_code_segment());
    let tss_selector = gdt.add_entry(Descriptor::tss_segment(tss));
    gdt.load();
    unsafe {
        CS::set_reg(code_selector);
        // set ss in that there is always a double fault when handler return
        SS::set_reg(SegmentSelector { 0: 0 });
        load_tss(tss_selector);
    }
}
