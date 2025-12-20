use crate::info;

pub mod apic;
pub mod gdt;
pub mod idt;
pub mod interrupts;
pub mod syscall;

pub fn init() {
    gdt::init();
    info!("GDT loaded");
    idt::init();
    info!("IDT loaded");
    syscall::init();
    info!("Syscalls loaded");
}

pub fn ticks() -> u64 {
    interrupts::ticks()
}
