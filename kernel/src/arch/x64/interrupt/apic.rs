use crate::interrupt::IrqVector;
use crate::memory::to_virt_addr;
// TODO RWLock
use spin::Mutex;
use x2apic::ioapic::IoApic;
use x2apic::lapic::{xapic_base, LocalApic, LocalApicBuilder, TimerMode};

pub static mut LAPIC: Option<Mutex<LocalApic>> = None;
pub static mut IOAPIC: Option<Mutex<IoApic>> = None;

pub const IOAPIC_OFFSET: u8 = 0x20;

pub unsafe fn eoi() {
    let mut lapic = LAPIC.as_mut().unwrap().lock();
    lapic.end_of_interrupt();
}

pub fn init() {
    let apic_physical_address = unsafe { xapic_base() };
    // TODO apic_physical_address 0xFEE00000 map the page and frame
    //      check if xapic_base is already in use in mmap
    //      if certain mapped frame in free memory region
    let apic_virtual_address = to_virt_addr(apic_physical_address).as_u64();
    unsafe {
        LAPIC = Some(Mutex::new(
            LocalApicBuilder::new()
                .timer_vector(IrqVector::Timer.with_offset())
                // FIXME
                .error_vector(IrqVector::Error.with_offset())
                .spurious_vector(IrqVector::Spurious.with_offset())
                .set_xapic_base(apic_virtual_address)
                .timer_initial(u32::MAX)
                .timer_mode(TimerMode::OneShot)
                .build()
                .unwrap_or_else(|err| panic!("{}", err)),
        ));
        let mut lapic = LAPIC.as_ref().unwrap().lock();
        lapic.enable();

        // TODO check if io apic regs addr is already in use in mmap
        // !!! Map the IOAPIC's MMIO address `addr` here !!!
        // let ioapic = IoApic::new(addr);
        IOAPIC = Some(Mutex::new(IoApic::new(to_virt_addr(0xFEC00000).as_u64())));
        let mut ioapic = IOAPIC.as_ref().unwrap().lock();
        ioapic.init(IOAPIC_OFFSET);
    };
}
