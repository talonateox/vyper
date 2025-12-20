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

    pub fn reap_dead(&mut self) {
        let mut i = self.tasks.len();
        while i > 0 {
            i -= 1;
            if i != self.current && self.tasks[i].state == TaskState::Dead {
                self.tasks.remove(i);
                if i < self.current {
                    self.current -= 1;
                }
            }
        }
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
        let switch_info = {
            let mut guard = SCHEDULER.lock();
            if let Some(sched) = guard.as_mut() {
                sched.reap_dead();

                let current_tick = crate::cpu::ticks();
                for task in sched.tasks.iter_mut() {
                    if task.state == TaskState::Sleeping {
                        if let Some(wake_at) = task.wake_at {
                            if current_tick >= wake_at {
                                task.state = TaskState::Ready;
                                task.wake_at = None;
                            }
                        }
                    }
                }

                let current = sched.current;

                if let Some(next) = sched.next_ready() {
                    sched.current = next;
                    if sched.tasks[current].state == TaskState::Running {
                        sched.tasks[current].state = TaskState::Ready;
                    }
                    sched.tasks[next].state = TaskState::Running;

                    let old_sp = &mut sched.tasks[current].stack_ptr as *mut u64;
                    let new_sp = sched.tasks[next].stack_ptr;
                    Some((old_sp, new_sp))
                } else {
                    None
                }
            } else {
                None
            }
        };

        if let Some((old_sp, new_sp)) = switch_info {
            unsafe { switch_context(old_sp, new_sp) };
        }
    });
}

pub fn yield_now() {
    schedule();
}

pub fn sleep(ticks: u64) {
    let current_tick = crate::cpu::ticks();

    x86_64::instructions::interrupts::without_interrupts(|| {
        if let Some(sched) = SCHEDULER.lock().as_mut() {
            sched.tasks[sched.current].wake_at = Some(current_tick + ticks);
            sched.tasks[sched.current].state = TaskState::Sleeping;
        }
    });
    schedule();
}

pub fn exit() {
    x86_64::instructions::interrupts::without_interrupts(|| {
        if let Some(sched) = SCHEDULER.lock().as_mut() {
            sched.tasks[sched.current].state = TaskState::Dead;
        }
    });

    schedule();

    loop {
        x86_64::instructions::hlt();
    }
}
