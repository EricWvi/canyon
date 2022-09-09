#![no_std]
#![cfg_attr(test, no_main)]
#![feature(custom_test_frameworks)]
#![test_runner(testing::test_runner)]
#![reexport_test_harness_main = "test_main"]
#![feature(alloc_error_handler)]
#![feature(abi_x86_interrupt)]

extern crate alloc;

mod allocator;

/// Architecture-dependent stuff
pub mod arch;
pub use crate::arch::*;

mod logger;
pub mod memory;
mod process;
#[cfg(feature = "qemu")]
pub mod testing;

use bootloader_lib::BootInfo;
use core::arch::asm;
use core::panic::PanicInfo;
use log::{error, info};

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    error!("{}", info);
    unsafe {
        loop {
            asm!("nop");
        }
    }
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    use crate::testing::{exit_qemu, QemuExitCode};
    error!("[failed]");
    error!("Error: {}", info);
    exit_qemu(QemuExitCode::Failed);
    loop {}
}

/// Entry point for `cargo test`
#[cfg(test)]
#[no_mangle]
pub extern "C" fn _start(boot_info: &'static BootInfo) -> ! {
    init(boot_info);

    test_main();
    unsafe {
        loop {
            asm!("nop");
        }
    }
}

pub fn init(boot_info: &'static BootInfo) {
    logger::init(boot_info.graphic_info);
    info!("enter kernel");
    info!("logger initialized");

    // ! The order cannot be changed.
    memory::init(boot_info.physical_memory_offset);
    gdt::init();
    idt::init();
    apic::init();
    x86_64::instructions::interrupts::enable();
}
