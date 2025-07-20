use crate::interrupt::{apic, IrqVector};
use x2apic::ioapic::{IoApic, IrqFlags, IrqMode};
use x86_64::instructions::port::Port;
use x86_64::structures::idt::InterruptStackFrame;

// 1 / (1.193182 MHz) = 838,095,110 femtoseconds ~= 838.095 ns
pub const PERIOD_FS: u128 = 838_095_110;

// (1.193182 MHz) / 1193 = 1000.151 Hz
pub const CHAN0_DIVISOR: u16 = 1193;

// Calculated interrupt period in nanoseconds based on divisor and period
pub const RATE: u128 = (CHAN0_DIVISOR as u128 * PERIOD_FS) / 1_000_000;

pub static mut CHAN0: Port<u8> = Port::new(0x40);
pub static mut CHAN1: Port<u8> = Port::new(0x41);
pub static mut CHAN2: Port<u8> = Port::new(0x42);
pub static mut CONTROL: Port<u8> = Port::new(0x43);

const MODE_2: u8 = 0b010 << 1;
const ACCESS_LATCH: u8 = 0b00 << 4;
// Bits 4 and 5 set the data channel mode to request two sequential 8-bit writes to the port
// (a single 16-bit write will not work, sadly).
const ACCESS_LOHI: u8 = 0b11 << 4;
const SELECT_CHAN0: u8 = 0b00 << 6;

static mut COUNT: usize = 0;

pub unsafe fn init() {
    CONTROL.write(SELECT_CHAN0 | ACCESS_LOHI | MODE_2);
    CHAN0.write(CHAN0_DIVISOR as u8);
    CHAN0.write((CHAN0_DIVISOR >> 8) as u8);
}

pub unsafe fn enable(ioapic: &mut IoApic, lapic_id: u8) {
    let mut entry = ioapic.table_entry(IrqVector::PIT.as_u8());
    entry.set_mode(IrqMode::Fixed);
    entry.set_flags(IrqFlags::MASKED);
    entry.set_dest(lapic_id);
    ioapic.set_table_entry(IrqVector::PIT.as_u8(), entry);
    ioapic.enable_irq(IrqVector::PIT.as_u8());
}

pub unsafe fn disable(ioapic: &mut IoApic) {
    ioapic.disable_irq(IrqVector::PIT.as_u8());
}

pub unsafe fn read() -> u16 {
    CONTROL.write(SELECT_CHAN0 | ACCESS_LATCH);
    let low = CHAN0.read();
    let high = CHAN0.read();
    let counter = ((high as u16) << 8) | (low as u16);
    // Counter is inverted, subtract from CHAN0_DIVISOR
    CHAN0_DIVISOR.saturating_sub(counter)
}

pub fn count() -> usize {
    unsafe { COUNT }
}

pub extern "x86-interrupt" fn pit_handler(_stack_frame: InterruptStackFrame) {
    unsafe {
        COUNT += 1;
        apic::eoi();
    }
}
