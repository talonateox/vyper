use spin::Lazy;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};

use crate::{cpu::gdt::DOUBLE_FAULT_IST_INDEX, info, print, println};

struct PrettyStackFrame<'a>(&'a InterruptStackFrame);

static IDT: Lazy<InterruptDescriptorTable> = Lazy::new(|| {
    let mut idt = InterruptDescriptorTable::new();
    idt.breakpoint.set_handler_fn(breakpoint_handler);
    unsafe {
        idt.double_fault
            .set_handler_fn(double_fault_handler)
            .set_stack_index(DOUBLE_FAULT_IST_INDEX);
    }
    idt
});

pub fn init() {
    IDT.load();
}

pub fn print_stack_frame(frame: InterruptStackFrame) {
    println!("  RIP: {:016x}", frame.instruction_pointer.as_u64());
    println!("  RSP: {:016x}", frame.stack_pointer.as_u64());
    println!("  RFL: {:016x}", frame.cpu_flags);
    println!("  CS:  {:04x}", frame.code_segment.0);
    print!("  SS:  {:04x}", frame.stack_segment.0)
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    info!("BREAKPOINT");
    print_stack_frame(stack_frame);
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    _error_code: u64,
) -> ! {
    println!("DOUBLE FAULT");
    print_stack_frame(stack_frame);
    panic!();
}
