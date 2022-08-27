#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

extern crate alloc;

mod allocator;
mod logger;

use crate::logger::init_logger;
use bootloader_lib::BootInfo;
use core::arch::asm;
use core::panic::PanicInfo;
use log::{debug, error, info};

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    error!("{}", info);
    unsafe {
        loop {
            asm!("nop");
        }
    }
}

#[no_mangle]
pub extern "C" fn _start(boot_info: &'static BootInfo) -> ! {
    init_logger(boot_info.graphic_info);

    unsafe {
        loop {
            asm!("nop");
        }
    }
}
