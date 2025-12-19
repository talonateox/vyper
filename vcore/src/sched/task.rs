use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TaskState {
    Ready,
    Running,
    Dead,
}

#[derive(Debug, Default, Clone)]
#[repr(C)]
pub struct Context {
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub rbx: u64,
    pub rbp: u64,
    pub rip: u64,
}

pub struct Task {
    pub id: u64,
    pub state: TaskState,
    pub context: Context,
    pub stack_ptr: u64,
    _stack: Vec<u8>,
}

impl Task {
    const STACK_SIZE: usize = 4096 * 4;

    pub fn new(entry: fn()) -> Self {
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);

        let mut stack = alloc::vec![0u8; Self::STACK_SIZE];

        let stack_top = stack.as_ptr() as u64 + Self::STACK_SIZE as u64;

        let stack_top = stack_top & !0xF;

        let context = Context {
            r15: 0,
            r14: 0,
            r13: 0,
            r12: 0,
            rbx: 0,
            rbp: 0,
            rip: entry as u64,
        };

        let stack_ptr = stack_top - core::mem::size_of::<Context>() as u64;

        unsafe {
            let ctx_ptr = stack_ptr as *mut Context;
            ctx_ptr.write(context.clone());
        }

        Self {
            id,
            state: TaskState::Ready,
            context,
            stack_ptr,
            _stack: stack,
        }
    }

    pub fn kernel_task() -> Self {
        Self {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            state: TaskState::Running,
            context: Context::default(),
            stack_ptr: 0,
            _stack: Vec::new(),
        }
    }
}
