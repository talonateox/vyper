use x86_64::{PhysAddr, VirtAddr, instructions::port::Port, structures::paging::PageTableFlags};

const LAPIC_VIRT: u64 = 0xFFFF_FFFF_FEE0_0000;
const LAPIC_PHYS: u64 = 0xFEE0_0000;

const IOAPIC_VIRT: u64 = 0xFFFF_FFFF_FEC0_0000;
const IOAPIC_PHYS: u64 = 0xFEC0_0000;

pub const LAPIC_ID: u64 = 0x020;
pub const LAPIC_EOI: u64 = 0x0B0;
pub const LAPIC_SPURIOUS: u64 = 0x0F0;
pub const LAPIC_TIMER_LVT: u64 = 0x320;
pub const LAPIC_TIMER_INIT: u64 = 0x380;
pub const LAPIC_TIMER_CURRENT: u64 = 0x390;
pub const LAPIC_TIMER_DIV: u64 = 0x3E0;

const IOAPIC_REG_SELECT: u64 = 0x00;
const IOAPIC_REG_DATA: u64 = 0x10;

const IOAPIC_ID: u32 = 0x00;
const IOAPIC_VER: u32 = 0x01;
const IOAPIC_REDTBL_BASE: u32 = 0x10;

const PIT_FREQ: u32 = 1193182;
const CALIBRATE_MS: u32 = 10;

static mut TICKS_PER_MS: u32 = 0;

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
    let ptr = (LAPIC_VIRT + offset) as *const u32;
    unsafe { core::ptr::read_volatile(ptr) }
}

pub unsafe fn lapic_write(offset: u64, value: u32) {
    let ptr = (LAPIC_VIRT + offset) as *mut u32;
    unsafe { core::ptr::write_volatile(ptr, value) };
}

pub unsafe fn ioapic_read(reg: u32) -> u32 {
    let select = IOAPIC_VIRT as *mut u32;
    let data = (IOAPIC_VIRT + IOAPIC_REG_DATA) as *mut u32;
    unsafe { core::ptr::write_volatile(select, reg) };
    unsafe { core::ptr::read_volatile(data) }
}

pub unsafe fn ioapic_write(reg: u32, value: u32) {
    let select = IOAPIC_VIRT as *mut u32;
    let data = (IOAPIC_VIRT + IOAPIC_REG_DATA) as *mut u32;
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

fn calibrate_timer() {
    unsafe {
        let mut pit_cmd: Port<u8> = Port::new(0x43);
        let mut pit_ch2: Port<u8> = Port::new(0x42);
        let mut pit_gate: Port<u8> = Port::new(0x61);

        let divisor = (PIT_FREQ / 1000) * CALIBRATE_MS;

        pit_cmd.write(0b10110010);

        let gate = pit_gate.read();
        pit_gate.write(gate | 0x01);

        pit_ch2.write((divisor & 0xFF) as u8);
        pit_ch2.write((divisor >> 8) as u8);

        lapic_write(LAPIC_TIMER_DIV, 0x3);
        lapic_write(LAPIC_TIMER_INIT, 0xFFFFFFFF);

        while pit_gate.read() & 0x20 == 0 {}

        let elapsed = 0xFFFFFFFF - lapic_read(LAPIC_TIMER_CURRENT);

        lapic_write(LAPIC_TIMER_INIT, 0);

        TICKS_PER_MS = elapsed / CALIBRATE_MS;
    }
}

pub fn init_keyboard_controller() {
    use x86_64::instructions::port::Port;

    unsafe {
        let mut cmd_port: Port<u8> = Port::new(0x64);
        let mut data_port: Port<u8> = Port::new(0x60);

        while cmd_port.read() & 0x02 != 0 {}

        cmd_port.write(0xAD);

        while cmd_port.read() & 0x02 != 0 {}

        cmd_port.write(0xA7);

        let _ = data_port.read();

        while cmd_port.read() & 0x02 != 0 {}
        cmd_port.write(0x20);
        while cmd_port.read() & 0x01 == 0 {}
        let mut config = data_port.read();

        config |= 0x01;
        config &= !0x40;

        while cmd_port.read() & 0x02 != 0 {}
        cmd_port.write(0x60);
        while cmd_port.read() & 0x02 != 0 {}
        data_port.write(config);

        while cmd_port.read() & 0x02 != 0 {}
        cmd_port.write(0xAE);

        while cmd_port.read() & 0x01 != 0 {
            let _ = data_port.read();
        }
    }
}

pub fn init() {
    disable_pic();

    let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_CACHE;

    // Map to high virtual addresses
    vmm::map_page(VirtAddr::new(LAPIC_VIRT), PhysAddr::new(LAPIC_PHYS), flags)
        .expect("failed to map lapic");

    vmm::map_page(
        VirtAddr::new(IOAPIC_VIRT),
        PhysAddr::new(IOAPIC_PHYS),
        flags,
    )
    .expect("failed to map ioapic");

    unsafe {
        lapic_write(LAPIC_SPURIOUS, 0x100 | SPURIOUS_VECTOR as u32);
    }

    calibrate_timer();

    let ticks_10ms = unsafe { TICKS_PER_MS * 10 };
    unsafe {
        lapic_write(LAPIC_TIMER_DIV, 0x3);
        lapic_write(LAPIC_TIMER_LVT, (1 << 17) | TIMER_VECTOR as u32);
        lapic_write(LAPIC_TIMER_INIT, ticks_10ms);
    }

    init_keyboard_controller();
    ioapic_set_irq(1, KEYBOARD_VECTOR);
}
