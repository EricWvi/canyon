#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points

use core::arch::asm;
use core::panic::PanicInfo;
use bootloader::BootInfo;

/// This function is called on panic.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

static HELLO: &[u8] = b"Hello, World!";

#[no_mangle]
pub extern "C" fn _start(boot_info: &'static BootInfo) -> i32 {
    12345
}

