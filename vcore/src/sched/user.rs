use alloc::vec::Vec;
use x86_64::{VirtAddr, structures::paging::PageTableFlags};

use crate::mem::vmm;

const USER_STACK_BASE: u64 = 0x7FFF_FFFF_0000;
const USER_STACK_SIZE: usize = 4096 * 4;

pub struct UserTask {
    pub entry: u64,
    pub stack_top: u64,
    _stack_pages: Vec<u64>,
}

impl UserTask {
    pub fn new(entry: u64, code: &[u8], code_addr: u64) -> Result<Self, &'static str> {
        let flags =
            PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE;

        let code_pages = (code.len() + 4095) / 4096;
        for i in 0..code_pages {
            let addr = VirtAddr::new(code_addr + (i * 4096) as u64);
            vmm::map_page_alloc(addr, flags)?;
        }

        unsafe {
            core::ptr::copy_nonoverlapping(code.as_ptr(), code_addr as *mut u8, code.len());
        }

        let mut stack_pages = Vec::new();
        let stack_pages_count = USER_STACK_SIZE / 4096;
        for i in 0..stack_pages_count {
            let addr = USER_STACK_BASE - (i as u64 * 4096);
            vmm::map_page_alloc(VirtAddr::new(addr), flags)?;
            stack_pages.push(addr);
        }

        let stack_top = USER_STACK_BASE + 4096 - 8;

        Ok(Self {
            entry,
            stack_top,
            _stack_pages: stack_pages,
        })
    }
}
