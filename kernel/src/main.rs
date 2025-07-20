#![no_std]
#![no_main]

use bootloader_lib::BootInfo;
use canyon::*;

#[no_mangle]
pub extern "C" fn _start(boot_info: &'static mut BootInfo) -> ! {
    init(boot_info);

    x86_64::instructions::hlt();
    loop {}
}
