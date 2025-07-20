use crate::device::pit;
use crate::interrupt::apic::LAPIC;
use log::{info, trace};
use x2apic::ioapic::IoApic;
use x2apic::lapic::TimerMode;

pub unsafe fn init(ioapic: &mut IoApic) {
    let pit_start = pit::count();
    x86_64::instructions::interrupts::enable();

    let lapic_start = {
        while pit::count() < pit_start + 2 {}
        let lapic = LAPIC.as_ref().unwrap().lock();
        lapic.timer_current()
    };
    let lapic_end = {
        // wait for 100ms
        while pit::count() < pit_start + 102 {}
        let lapic = LAPIC.as_ref().unwrap().lock();
        lapic.timer_current()
    };
    pit::disable(&mut *ioapic);
    trace!("pit disabled");

    let count = lapic_start - lapic_end;
    trace!("lapic count {:?} in 100ms", count);
    set_apic_timer(count);
    info!("set apic timer");
}

unsafe fn set_apic_timer(divisor: u32) {
    let mut lapic = LAPIC.as_ref().unwrap().lock();
    lapic.disable_timer();
    lapic.set_timer_mode(TimerMode::Periodic);
    lapic.set_timer_initial(divisor);
    lapic.enable_timer();
}

static mut COUNT: usize = 0;

pub unsafe fn increment() {
    COUNT += 1;
}

pub fn count() -> usize {
    unsafe { COUNT }
}

pub fn sleep(sec: i32) {
    unsafe {
        let end = COUNT + (10 * sec) as usize;
        while COUNT < end {}
    }
}
