use crate::interrupt::{InterruptIndex, IrqVector};
use crate::memory::to_virt_addr;
use alloc::boxed::Box;
use alloc::vec::Vec;
use log::info;
use spin::Mutex;
use x2apic::ioapic::{IoApic, IrqFlags, IrqMode, RedirectionTableEntry};
use x2apic::lapic::{xapic_base, LocalApic, LocalApicBuilder};

pub static mut LAPIC: Option<Mutex<LocalApic>> = None;
pub static mut IOAPIC: Option<Mutex<IoApic>> = None;

pub const IOAPIC_OFFSET: u8 = 0x20;

pub fn init() {
    let apic_physical_address = unsafe { xapic_base() };
    // TODO apic_physical_address 0xFEE00000 map the page and frame
    //      check if xapic_base is already in use in mmap
    let apic_virtual_address = to_virt_addr(apic_physical_address).as_u64();
    unsafe {
        LAPIC = Some(Mutex::new(
            LocalApicBuilder::new()
                .timer_vector(InterruptIndex::Timer.as_usize())
                // FIXME
                .error_vector(InterruptIndex::Error.as_usize())
                .spurious_vector(InterruptIndex::Spurious.as_usize())
                .set_xapic_base(apic_virtual_address)
                .build()
                .unwrap_or_else(|err| panic!("{}", err)),
        ));
        let mut lapic = LAPIC.as_ref().unwrap().lock();
        lapic.enable();

        let apic_id = lapic.id();
        // TODO check if io apic regs addr is already in use in mmap
        IOAPIC = Some(Mutex::new(IoApic::new(to_virt_addr(0xFEC00000).as_u64())));
        let mut ioapic = IOAPIC.as_ref().unwrap().lock();

        ioapic.init(IOAPIC_OFFSET);

        // TODO move to `drivers`
        let mut entry = ioapic.table_entry(IrqVector::Keyboard.as_u8());
        entry.set_mode(IrqMode::Fixed);
        entry.set_flags(IrqFlags::MASKED);
        entry.set_dest(apic_id as u8);
        ioapic.set_table_entry(IrqVector::Keyboard.as_u8(), entry);

        ioapic.enable_irq(IrqVector::Keyboard.as_u8());
    };
}
