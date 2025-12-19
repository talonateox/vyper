use x86_64::{PhysAddr, VirtAddr, instructions::port::Port, structures::paging::PageTableFlags};

const LAPIC_BASE: u64 = 0xFEE0_0000;

pub const LAPIC_ID: u64 = 0x020;
pub const LAPIC_EOI: u64 = 0x0B0;
pub const LAPIC_SPURIOUS: u64 = 0x0F0;
pub const LAPIC_TIMER_LVT: u64 = 0x320;
pub const LAPIC_TIMER_INIT: u64 = 0x380;
pub const LAPIC_TIMER_CURRENT: u64 = 0x390;
pub const LAPIC_TIMER_DIV: u64 = 0x3E0;

const IOAPIC_BASE: u64 = 0xFEC0_0000;
const IOAPIC_REG_SELECT: u64 = 0x00;
const IOAPIC_REG_DATA: u64 = 0x10;

const IOAPIC_ID: u32 = 0x00;
const IOAPIC_VER: u32 = 0x01;
const IOAPIC_REDTBL_BASE: u32 = 0x10;

use crate::{cpu::interrupts::KEYBOARD_VECTOR, mem::vmm};

use super::interrupts::{SPURIOUS_VECTOR, TIMER_VECTOR};

pub fn disable_pic() {
    unsafe {
        let mut pic1_cmd: Port<u8> = Port::new(0x20);
        let mut pic1_data: Port<u8> = Port::new(0x21);
        let mut pic2_cmd: Port<u8> = Port::new(0xA0);
        let mut pic2_data: Port<u8> = Port::new(0xA1);

        pic1_cmd.write(0x11);
        pic2_cmd.write(0x11);

        pic1_data.write(0x20);
        pic2_data.write(0x28);

        pic1_data.write(4);
        pic2_data.write(2);

        pic1_data.write(0x01);
        pic2_data.write(0x01);

        pic1_data.write(0xFF);
        pic2_data.write(0xFF);
    }
}

pub unsafe fn lapic_read(offset: u64) -> u32 {
    let ptr = (LAPIC_BASE + offset) as *const u32;
    unsafe { core::ptr::read_volatile(ptr) }
}

pub unsafe fn lapic_write(offset: u64, value: u32) {
    let ptr = (LAPIC_BASE + offset) as *mut u32;
    unsafe { core::ptr::write_volatile(ptr, value) };
}

pub unsafe fn ioapic_read(reg: u32) -> u32 {
    let select = IOAPIC_BASE as *mut u32;
    let data = (IOAPIC_BASE + IOAPIC_REG_DATA) as *mut u32;
    unsafe { core::ptr::write_volatile(select, reg) };
    unsafe { core::ptr::read_volatile(data) }
}

pub unsafe fn ioapic_write(reg: u32, value: u32) {
    let select = IOAPIC_BASE as *mut u32;
    let data = (IOAPIC_BASE + IOAPIC_REG_DATA) as *mut u32;
    unsafe { core::ptr::write_volatile(select, reg) };
    unsafe { core::ptr::write_volatile(data, value) };
}

fn ioapic_set_irq(irq: u8, vector: u8) {
    let redtbl_reg = IOAPIC_REDTBL_BASE + (irq as u32 * 2);

    unsafe {
        ioapic_write(redtbl_reg, vector as u32);
        ioapic_write(redtbl_reg + 1, 0);
    }
}

pub fn init() {
    disable_pic();

    let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_CACHE;
    vmm::map_page(VirtAddr::new(LAPIC_BASE), PhysAddr::new(LAPIC_BASE), flags)
        .expect("failed to map lapic");

    vmm::map_page(
        VirtAddr::new(IOAPIC_BASE),
        PhysAddr::new(IOAPIC_BASE),
        flags,
    )
    .expect("failed to map ioapic");

    unsafe {
        lapic_write(LAPIC_SPURIOUS, 0x100 | SPURIOUS_VECTOR as u32);
        lapic_write(LAPIC_TIMER_DIV, 0x3);
        lapic_write(LAPIC_TIMER_LVT, (1 << 17) | TIMER_VECTOR as u32);
        lapic_write(LAPIC_TIMER_INIT, 0x10000);
    }

    ioapic_set_irq(1, KEYBOARD_VECTOR);
}
