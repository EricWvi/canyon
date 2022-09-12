pub mod apic;
pub mod idt;

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = 32,
    Keyboard = 33,
    Error = 60,
    Spurious = 61,
}

impl InterruptIndex {
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    pub fn as_usize(self) -> usize {
        usize::from(self.as_u8())
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum IrqVector {
    Keyboard = 1,
}

impl IrqVector {
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    pub fn as_usize(self) -> usize {
        usize::from(self.as_u8())
    }
}
