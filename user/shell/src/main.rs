#![no_std]
#![no_main]

mod commands;
mod input;

use vlib::{print, println, syscalls::exit};

use crate::{commands::execute, input::read_line};

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    println!("herro, welcome to vshell(vyper shell)");

    let mut buf = [0u8; 256];

    loop {
        print!("> ");

        let len = read_line(&mut buf);
        let line = &buf[..len];

        if len == 0 {
            continue;
        }

        execute(line);
    }
}
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    exit(1);
}
