pub mod apic;
pub mod idt;

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum IrqVector {
    Keyboard = 1,
    PIT = 2,
    Timer = 16,
    Error = 28,
    Spurious = 29,
}

impl IrqVector {
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    pub fn with_offset(self) -> usize {
        usize::from(self as u8 + apic::IOAPIC_OFFSET)
    }
}
