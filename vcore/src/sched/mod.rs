pub mod switch;
pub mod task;

use alloc::collections::VecDeque;
use spin::Mutex;
use switch::switch_context;
use task::{Task, TaskState};

pub static SCHEDULER: Mutex<Option<Scheduler>> = Mutex::new(None);

pub struct Scheduler {
    tasks: VecDeque<Task>,
    current: usize,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            tasks: VecDeque::new(),
            current: 0,
        }
    }

    pub fn add_task(&mut self, task: Task) {
        self.tasks.push_back(task);
    }

    fn next_ready(&self) -> Option<usize> {
        let len = self.tasks.len();
        for i in 1..=len {
            let idx = (self.current + i) % len;
            if self.tasks[idx].state == TaskState::Ready {
                return Some(idx);
            }
        }
        None
    }

    pub unsafe fn switch_to(&mut self, next: usize) {
        if next == self.current {
            return;
        }

        let current = self.current;
        self.current = next;

        self.tasks[current].state = TaskState::Ready;
        self.tasks[next].state = TaskState::Running;

        let old_sp = &mut self.tasks[current].stack_ptr as *mut u64;
        let new_sp = self.tasks[next].stack_ptr;

        unsafe { switch_context(old_sp, new_sp) };
    }
}

pub fn init() {
    let mut sched = Scheduler::new();
    sched.add_task(Task::kernel_task());
    *SCHEDULER.lock() = Some(sched);
}

pub fn spawn(entry: fn()) {
    let task = Task::new(entry);
    if let Some(sched) = SCHEDULER.lock().as_mut() {
        sched.add_task(task);
    }
}

pub fn schedule() {
    x86_64::instructions::interrupts::without_interrupts(|| {
        let mut guard = SCHEDULER.lock();
        if let Some(sched) = guard.as_mut() {
            if let Some(next) = sched.next_ready() {
                unsafe {
                    sched.switch_to(next);
                }
            }
        }
    });
}
