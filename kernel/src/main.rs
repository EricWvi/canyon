#![no_std]
#![no_main]

use bootloader_lib::BootInfo;
use canyon::*;
use core::arch::asm;
use log::{debug, info};

#[no_mangle]
pub extern "C" fn _start(boot_info: &'static BootInfo) -> ! {
    init(boot_info);

    unsafe {
        loop {
            asm!("nop");
        }
    }
}
