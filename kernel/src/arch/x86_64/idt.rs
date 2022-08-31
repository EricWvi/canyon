use crate::gdt;
use log::error;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};

static mut IDT: Option<InterruptDescriptorTable> = None;

pub fn init() {
    let mut idt = unsafe {
        IDT = Some(InterruptDescriptorTable::new());
        IDT.as_mut().unwrap()
    };
    idt.breakpoint.set_handler_fn(breakpoint_handler);
    unsafe {
        idt.double_fault
            .set_handler_fn(double_fault_handler)
            .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
    }
    // To prevent triple faults in all cases, we also set up an Interrupt Stack Table
    // to catch double faults on a separate kernel stack.

    // A guard page is a special memory page at the bottom of a stack that
    // makes it possible to detect stack overflows. The page is not
    // mapped to any physical frame, so accessing it causes a page fault instead of
    // silently corrupting other memory. The bootloader sets up a guard page
    // for our kernel stack, so a stack overflow causes a page fault.
    idt.load();
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    error!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) -> ! {
    error!("EXCEPTION: DOUBLE_FAULT");
    error!("error_code: {}", error_code);
    panic!("{:#?}", stack_frame);
}

#[cfg(test)]
mod test {
    use crate::testing::*;
    use log::info;

    #[test_case]
    fn test_breakpoint() {
        x86_64::instructions::interrupts::int3();
        info!("after int3");
    }
}
