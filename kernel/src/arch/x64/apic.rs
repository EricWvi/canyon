use crate::memory::to_virt_addr;
use spin::Mutex;
use x2apic::lapic::{xapic_base, LocalApic, LocalApicBuilder};

pub static mut LAPIC: Option<Mutex<LocalApic>> = None;

pub fn init() {
    let apic_physical_address = unsafe { xapic_base() };
    let apic_virtual_address = to_virt_addr(apic_physical_address).as_u64();
    unsafe {
        LAPIC = Some(Mutex::new(
            LocalApicBuilder::new()
                .timer_vector(crate::interrupt::InterruptIndex::Timer.as_usize())
                // FIXME
                .error_vector(33)
                .spurious_vector(34)
                .set_xapic_base(apic_virtual_address)
                .build()
                .unwrap_or_else(|err| panic!("{}", err)),
        ));
        let mut lapic = LAPIC.as_ref().unwrap().lock();
        lapic.enable();
    };
}
