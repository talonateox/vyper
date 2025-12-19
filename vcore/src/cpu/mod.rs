use crate::info;

pub mod gdt;
pub mod idt;

pub fn init() {
    gdt::init();
    info!("GDT loaded");
    idt::init();
    info!("IDT loaded")
}
