use crate::device::pit;
use crate::interrupt::apic;
use crate::interrupt::IrqVector;
use crate::{device, gdt};
use log::error;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};

static mut IDT: Option<InterruptDescriptorTable> = None;

pub fn init() {
    let idt = unsafe {
        IDT = Some(InterruptDescriptorTable::new());
        IDT.as_mut().unwrap()
    };
    idt.divide_error.set_handler_fn(divide_error_handler);
    idt.debug.set_handler_fn(debug_handler);
    idt.non_maskable_interrupt
        .set_handler_fn(non_maskable_interrupt_handler);
    idt.breakpoint.set_handler_fn(breakpoint_handler);
    idt.overflow.set_handler_fn(overflow_handler);
    idt.bound_range_exceeded
        .set_handler_fn(bound_range_exceeded_handler);
    idt.invalid_opcode.set_handler_fn(invalid_opcode_handler);
    idt.device_not_available
        .set_handler_fn(device_not_available_handler);
    unsafe {
        idt.double_fault
            .set_handler_fn(double_fault_handler)
            .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
    }
    idt.invalid_tss.set_handler_fn(invalid_tss_handler);
    idt.segment_not_present
        .set_handler_fn(segment_not_present_handler);
    idt.stack_segment_fault
        .set_handler_fn(stack_segment_fault_handler);
    idt.general_protection_fault
        .set_handler_fn(general_protection_fault_handler);
    idt.page_fault.set_handler_fn(page_fault_handler);
    idt.x87_floating_point
        .set_handler_fn(x87_floating_point_handler);
    idt.alignment_check.set_handler_fn(alignment_check_handler);
    idt.machine_check.set_handler_fn(machine_check_handler);
    idt.simd_floating_point
        .set_handler_fn(simd_floating_point_handler);
    idt.virtualization.set_handler_fn(virtualization_handler);
    idt.vmm_communication_exception
        .set_handler_fn(vmm_communication_exception_handler);
    idt.security_exception
        .set_handler_fn(security_exception_handler);

    idt[IrqVector::PIT.with_offset()].set_handler_fn(pit::pit_handler);
    idt[IrqVector::Timer.with_offset()].set_handler_fn(timer_interrupt_handler);
    idt[IrqVector::Keyboard.with_offset()].set_handler_fn(keyboard_interrupt_handler);
    idt[IrqVector::Error.with_offset()].set_handler_fn(apic_error_handler);
    idt[IrqVector::Spurious.with_offset()].set_handler_fn(spurious_interrupt_handler);

    // TODO A guard page is a special memory page at the bottom of a stack that
    //      makes it possible to detect stack overflows. The page is not
    //      mapped to any physical frame, so accessing it causes a page fault instead of
    //      silently corrupting other memory. The bootloader sets up a guard page
    //      for our kernel stack, so a stack overflow causes a page fault.
    idt.load();
}

extern "x86-interrupt" fn divide_error_handler(stack_frame: InterruptStackFrame) {
    error!("EXCEPTION: DIVIDE_ERROR\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn debug_handler(stack_frame: InterruptStackFrame) {
    error!("EXCEPTION: DEBUG\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn non_maskable_interrupt_handler(stack_frame: InterruptStackFrame) {
    error!("EXCEPTION: NON_MASKABLE_INTERRUPT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    error!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn overflow_handler(stack_frame: InterruptStackFrame) {
    error!("EXCEPTION: OVERFLOW\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn bound_range_exceeded_handler(stack_frame: InterruptStackFrame) {
    error!("EXCEPTION: BOUND_RANGE_EXCEEDED\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn invalid_opcode_handler(stack_frame: InterruptStackFrame) {
    error!("EXCEPTION: INVALID_OPCODE\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn device_not_available_handler(stack_frame: InterruptStackFrame) {
    error!("EXCEPTION: DEVICE_NOT_AVAILABLE\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) -> ! {
    error!("EXCEPTION: DOUBLE_FAULT");
    error!("error_code: {}", error_code);
    panic!("{:#?}", stack_frame);
}

extern "x86-interrupt" fn invalid_tss_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    error!("EXCEPTION: INVALID_TSS");
    error!("error_code: {}", error_code);
    error!("{:#?}", stack_frame);
}

extern "x86-interrupt" fn segment_not_present_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    error!("EXCEPTION: SEGMENT_NOT_PRESENT");
    error!("error_code: {}", error_code);
    error!("{:#?}", stack_frame);
}

extern "x86-interrupt" fn stack_segment_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    error!("EXCEPTION: STACK_SEGMENT_FAULT");
    error!("error_code: {}", error_code);
    error!("{:#?}", stack_frame);
}

extern "x86-interrupt" fn general_protection_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    error!("EXCEPTION: GENERAL_PROTECTION_FAULT");
    error!("error_code: {}", error_code);
    error!("{:#?}", stack_frame);
}

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    error!("EXCEPTION: PAGE_FAULT\n{:#?}", stack_frame);
    error!("error_code: {:#?}", error_code);
}

extern "x86-interrupt" fn x87_floating_point_handler(stack_frame: InterruptStackFrame) {
    error!("EXCEPTION: X87_FLOATING_POINT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn alignment_check_handler(
    stack_frame: InterruptStackFrame,
    _error_code: u64,
) {
    error!("EXCEPTION: ALIGNMENT_CHECK\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn machine_check_handler(stack_frame: InterruptStackFrame) -> ! {
    panic!("EXCEPTION: MACHINE_CHECK\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn simd_floating_point_handler(stack_frame: InterruptStackFrame) {
    error!("EXCEPTION: SIMD_FLOATING_POINT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn virtualization_handler(stack_frame: InterruptStackFrame) {
    error!("EXCEPTION: VIRTUALIZATION\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn vmm_communication_exception_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    error!("EXCEPTION: VMM_COMMUNICATION_EXCEPTION\n{:#?}", stack_frame);
    error!("error_code: {:#?}", error_code);
}

extern "x86-interrupt" fn security_exception_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    error!("EXCEPTION: SECURITY_EXCEPTION\n{:#?}", stack_frame);
    error!("error_code: {:#?}", error_code);
}

extern "x86-interrupt" fn apic_error_handler(stack_frame: InterruptStackFrame) {
    error!("EXCEPTION: APIC_ERROR\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn spurious_interrupt_handler(stack_frame: InterruptStackFrame) {
    error!("EXCEPTION: SPURIOUS_INTERRUPT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    // error!("EXCEPTION: TIMER_INTERRUPT");
    unsafe {
        device::timer::increment();
        apic::eoi();
    }
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    use x86_64::instructions::port::Port;

    let mut port = Port::new(0x60);
    let _scancode: u8 = unsafe { port.read() };
    // info!("{}", scancode);
    // TODO use spin lock in interrupt handler will probably cause deadlock
    unsafe {
        apic::eoi();
    }
}

#[cfg(test)]
mod test {
    use crate::interrupt::apic::LAPIC;
    use crate::testing::*;
    use core::arch::x86_64::{__cpuid, _rdtsc};
    use log::{debug, info};

    #[test_case]
    fn test_breakpoint() {
        x86_64::instructions::interrupts::int3();
        info!("after int3");
    }
}
