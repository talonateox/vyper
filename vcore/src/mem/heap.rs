use linked_list_allocator::LockedHeap;
use x86_64::{VirtAddr, structures::paging::PageTableFlags};

use crate::mem::{PAGE_SIZE, vmm};

const HEAP_START: u64 = 0xFFFF_8080_0000_0000;
const HEAP_SIZE: usize = 1024 * 1024;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

pub fn init() -> Result<(), &'static str> {
    let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_EXECUTE;

    let heap_pages = (HEAP_SIZE + PAGE_SIZE - 1) / PAGE_SIZE;
    for i in 0..heap_pages {
        let addr = VirtAddr::new(HEAP_START + (i * PAGE_SIZE) as u64);
        vmm::map_page_alloc(addr, flags)?;
    }

    unsafe {
        ALLOCATOR.lock().init(HEAP_START as *mut u8, HEAP_SIZE);
    }

    Ok(())
}

pub fn size() -> usize {
    HEAP_SIZE / 1024
}
