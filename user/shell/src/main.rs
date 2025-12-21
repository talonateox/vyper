#![no_std]
#![no_main]

use vlib::syscalls::{exit, write};

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    write(1, b"Hello world!");
    exit(0);
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    exit(1);
}
