use crate::interrupt::IrqVector;
use x2apic::ioapic::{IoApic, IrqFlags, IrqMode};

pub unsafe fn init(ioapic: &mut IoApic, apic_id: u8) {
    let mut entry = ioapic.table_entry(IrqVector::Keyboard.as_u8());
    entry.set_mode(IrqMode::Fixed);
    entry.set_flags(IrqFlags::MASKED);
    entry.set_dest(apic_id);
    ioapic.set_table_entry(IrqVector::Keyboard.as_u8(), entry);

    ioapic.enable_irq(IrqVector::Keyboard.as_u8());
}
