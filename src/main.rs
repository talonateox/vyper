#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

use crate::{logger::Logger, vga::ColorCode};
use core::panic::PanicInfo;
use log::{error, info};

mod gdt;
mod interrupts;
mod logger;
mod vga;

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    vga::set_color(ColorCode::new(vga::Color::Green, vga::Color::Black));
    println!("      _                   .-=-.          .-==-.");
    println!("     {{ }}      __        .' O o '.       /  -<' )");
    println!("     {{ }}    .' O'.     / o .-. O \\     /  .--v`");
    println!("     {{ }}   / .-. o\\   /O  /   \\  o\\   /O /");
    println!("      \\ `-` /   \\ O`-'o  /     \\  O`-`o /");
    println!("       `-.-`     '.____.'       `.____.'");
    vga::set_color(ColorCode::new(vga::Color::White, vga::Color::Black));
    let _ = logger::init(log::LevelFilter::Debug);

    info!("Loading GDT");
    gdt::init();
    info!("Loading IDT");
    interrupts::init();

    loop {}
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    error!("{}", info);
    loop {}
}
