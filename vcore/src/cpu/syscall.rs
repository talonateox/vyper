use core::arch::{asm, naked_asm};

use x86_64::{
    VirtAddr,
    registers::{
        control::{Efer, EferFlags},
        model_specific::{KernelGsBase, LStar, SFMask, Star},
        rflags::RFlags,
    },
};

use crate::{cpu::gdt, error, info, print, sched};

#[repr(C, align(16))]
struct CpuLocal {
    user_rsp: u64,
    kernel_rsp: u64,
}

static mut CPU_LOCAL: CpuLocal = CpuLocal {
    user_rsp: 0,
    kernel_rsp: 0,
};

static mut SYSCALL_STACK: [u8; 4096 * 4] = [0; 4096 * 4];

pub fn init() {
    let selectors = gdt::selectors();

    unsafe {
        Star::write(
            selectors.user_code,
            selectors.user_data,
            selectors.kernel_code,
            selectors.kernel_data,
        )
        .expect("failed to set STAR");

        LStar::write(VirtAddr::new(syscall_entry as *const () as u64));
        SFMask::write(RFlags::INTERRUPT_FLAG);

        let efer = Efer::read();
        Efer::write(efer | EferFlags::SYSTEM_CALL_EXTENSIONS);

        let cpu_local_addr = &CPU_LOCAL as *const _ as u64;
        CPU_LOCAL.kernel_rsp = SYSCALL_STACK.as_ptr() as u64 + SYSCALL_STACK.len() as u64;

        KernelGsBase::write(VirtAddr::new(cpu_local_addr));
    }
}

#[unsafe(naked)]
unsafe extern "C" fn syscall_entry() {
    naked_asm!(
        "swapgs",
        "mov gs:[0], rsp",
        "mov rsp, gs:[8]",

        "push rcx",
        "push r11",
        "push rax",

        "push rdi",
        "push rsi",
        "push rdx",
        "push r10",
        "push r8",
        "push r9",
        "push rbx",
        "push rbp",
        "push r12",
        "push r13",
        "push r14",
        "push r15",

        "mov rdi, rax",
        "mov rsi, [rsp + 11*8]",
        "mov rdx, [rsp + 10*8]",
        "mov rcx, [rsp + 9*8]",
        "mov r8, [rsp + 8*8]",
        "mov r9, [rsp + 7*8]",

        "call {handler}",

        "pop r15",
        "pop r14",
        "pop r13",
        "pop r12",
        "pop rbp",
        "pop rbx",
        "pop r9",
        "pop r8",
        "pop r10",
        "pop rdx",
        "pop rsi",
        "pop rdi",

        "add rsp, 8",
        "pop r11",
        "pop rcx",

        "mov rsp, gs:[0]",
        "swapgs",

        "sysretq",
        handler = sym syscall_handler,
    );
}

extern "C" fn syscall_handler(
    num: u64,
    arg1: u64,
    arg2: u64,
    arg3: u64,
    arg4: u64,
    arg5: u64,
) -> u64 {
    match num {
        0 => {
            info!("task exited with code {}", arg1);
            sched::exit();
            0
        }
        1 => {
            let buf = arg2 as *const u8;
            let len = arg3 as usize;

            for i in 0..len {
                let c = unsafe { *buf.add(i) };
                print!("{}", c as char);
            }
            len as u64
        }
        _ => {
            error!("unknown syscall: {}", num);
            u64::MAX
        }
    }
}

pub unsafe fn jump_to_usermode(entry: u64, user_stack: u64) -> ! {
    let selectors = gdt::selectors();

    let user_cs = selectors.user_code.0 as u64;
    let user_ss = selectors.user_data.0 as u64;

    unsafe {
        asm!(
            "push {user_ss}",
            "push {user_stack}",
            "push 0x202",
            "push {user_cs}",
            "push {entry}",
            "iretq",
            user_ss = in(reg) user_ss,
            user_stack = in(reg) user_stack,
            user_cs = in(reg) user_cs,
            entry = in(reg) entry,
            options(noreturn)
        )
    }
}
