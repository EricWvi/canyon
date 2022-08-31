#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(canyon::testing::test_runner)]
#![reexport_test_harness_main = "test_main"]

use bootloader_lib::BootInfo;
use canyon::*;
use core::arch::asm;

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

#[test_case]
fn test_stack_overflow() {
    fn stack_overflow() {
        stack_overflow();
    }
    stack_overflow();
}
