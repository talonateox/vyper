use alloc::vec::Vec;
use core::{
    arch::naked_asm,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TaskState {
    Ready,
    Running,
    Sleeping,
    Dead,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TaskMode {
    Kernel,
    User,
}

pub struct Task {
    pub id: u64,
    pub state: TaskState,
    pub mode: TaskMode,
    pub stack_ptr: u64,
    pub wake_at: Option<u64>,
    pub user_entry: u64,
    pub user_stack: u64,
    _stack: Vec<u8>,
}

#[allow(improper_ctypes_definitions)]
extern "C" fn entry_wrapper(entry: fn()) -> ! {
    x86_64::instructions::interrupts::enable();

    entry();
    super::exit();
    unreachable!();
}

extern "C" fn user_entry_wrapper(entry: u64, stack: u64) -> ! {
    x86_64::instructions::interrupts::enable();
    unsafe {
        crate::cpu::syscall::jump_to_usermode(entry, stack);
    }
}

#[unsafe(naked)]
unsafe extern "C" fn entry_trampoline() -> ! {
    naked_asm!(
        "mov rdi, r15",
        "call {wrapper}",
        "ud2",
        wrapper = sym entry_wrapper,
    );
}

#[unsafe(naked)]
unsafe extern "C" fn user_entry_trampoline() -> ! {
    core::arch::naked_asm!(
        "mov rdi, r15",
        "mov rsi, r14",
        "call {wrapper}",
        "ud2",
        wrapper = sym user_entry_wrapper,
    );
}

impl Task {
    const STACK_SIZE: usize = 4096 * 4;

    pub fn new(entry: fn()) -> Self {
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let stack = alloc::vec![0u8; Self::STACK_SIZE];

        let stack_top = stack.as_ptr() as u64 + Self::STACK_SIZE as u64;
        let stack_top = stack_top & !0xF;

        let mut sp = stack_top;

        unsafe {
            sp -= 8;
            (sp as *mut u64).write(entry_trampoline as *const () as u64);

            sp -= 8;
            (sp as *mut u64).write(0);

            sp -= 8;
            (sp as *mut u64).write(0);

            sp -= 8;
            (sp as *mut u64).write(0);

            sp -= 8;
            (sp as *mut u64).write(0);

            sp -= 8;
            (sp as *mut u64).write(0);

            sp -= 8;
            (sp as *mut u64).write(entry as u64);
        }

        Self {
            id,
            state: TaskState::Ready,
            mode: TaskMode::Kernel,
            stack_ptr: sp,
            wake_at: None,
            user_entry: 0,
            user_stack: 0,
            _stack: stack,
        }
    }

    pub fn new_user(user_entry: u64, user_stack: u64) -> Self {
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let stack = alloc::vec![0u8; Self::STACK_SIZE];

        let stack_top = stack.as_ptr() as u64 + Self::STACK_SIZE as u64;
        let stack_top = stack_top & !0xF;

        let mut sp = stack_top;

        unsafe {
            sp -= 8;
            (sp as *mut u64).write(user_entry_trampoline as *const () as u64);
            sp -= 8;
            (sp as *mut u64).write(0);
            sp -= 8;
            (sp as *mut u64).write(0);
            sp -= 8;
            (sp as *mut u64).write(0);
            sp -= 8;
            (sp as *mut u64).write(0);
            sp -= 8;
            (sp as *mut u64).write(user_stack);
            sp -= 8;
            (sp as *mut u64).write(user_entry);
        }

        Self {
            id,
            state: TaskState::Ready,
            mode: TaskMode::User,
            stack_ptr: sp,
            wake_at: None,
            user_entry,
            user_stack,
            _stack: stack,
        }
    }

    pub fn kernel_task() -> Self {
        Self {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            state: TaskState::Running,
            mode: TaskMode::Kernel,
            stack_ptr: 0,
            wake_at: None,
            user_entry: 0,
            user_stack: 0,
            _stack: Vec::new(),
        }
    }
}
