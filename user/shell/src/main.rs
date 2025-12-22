#![no_std]
#![no_main]

mod commands;
mod input;

use vlib::{
    as_str, print, println,
    syscalls::{exit, getcwd},
};

use crate::{commands::execute, input::read_line};

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    println!("herro, welcome to vshell(vyper shell)");

    let mut buf = [0u8; 256];

    loop {
        let mut cwd = [0u8; 256];
        let cwd_len = getcwd(&mut cwd) as usize;
        let cwd = as_str!(&cwd[..cwd_len]);
        print!("[{}] ", cwd);

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
