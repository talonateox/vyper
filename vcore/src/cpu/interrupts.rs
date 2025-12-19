use core::sync::atomic::{AtomicU64, Ordering};

use spin::Mutex;
use x86_64::structures::idt::InterruptStackFrame;

use crate::{
    cpu::apic::{self, LAPIC_EOI},
    sched,
};

pub const TIMER_VECTOR: u8 = 32;
pub const KEYBOARD_VECTOR: u8 = 33;
pub const SPURIOUS_VECTOR: u8 = 255;

static TICKS: AtomicU64 = AtomicU64::new(0);

pub extern "x86-interrupt" fn timer_handler(_stack_frame: InterruptStackFrame) {
    TICKS.fetch_add(1, Ordering::Relaxed);

    unsafe {
        end_of_interrupt();
    }
}

pub extern "x86-interrupt" fn keyboard_handler(_stack_frame: InterruptStackFrame) {
    use x86_64::instructions::port::Port;

    let mut port = Port::new(0x60);
    let scancode: u8 = unsafe { port.read() };

    unsafe {
        end_of_interrupt();
    }

    sched::schedule();
}

pub extern "x86-interrupt" fn spurious_handler(_stack_frame: InterruptStackFrame) {}

pub fn ticks() -> u64 {
    TICKS.load(Ordering::Relaxed)
}

unsafe fn end_of_interrupt() {
    unsafe { apic::lapic_write(LAPIC_EOI, 0) };
}
