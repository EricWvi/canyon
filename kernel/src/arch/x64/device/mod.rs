use crate::interrupt::apic::{IOAPIC, LAPIC};
use log::info;

pub mod keyboard;
pub mod pit;
pub mod timer;

pub fn init() {
    // set up keyboard
    unsafe {
        let mut ioapic = IOAPIC.as_ref().unwrap().lock();
        let lapic = LAPIC.as_ref().unwrap().lock();
        keyboard::init(&mut *ioapic, lapic.id() as u8);
    }
    unsafe {
        pit::init();
        let mut ioapic = IOAPIC.as_ref().unwrap().lock();
        let lapic = LAPIC.as_ref().unwrap().lock();
        pit::enable(&mut *ioapic, lapic.id() as u8);
        info!("pit set up");
    }
    // set up timer and **also** enable interrupt
    unsafe {
        let mut ioapic = IOAPIC.as_ref().unwrap().lock();
        timer::init(&mut *ioapic);
    }
}
