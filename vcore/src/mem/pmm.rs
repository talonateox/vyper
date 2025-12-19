use limine::memory_map::{Entry, EntryType};
use spin::Mutex;
use x86_64::VirtAddr;

const PAGE_SIZE: usize = 4096;

static PMM: Mutex<Option<BitmapAllocator>> = Mutex::new(None);

struct BitmapAllocator {
    bitmap: *mut u8,
    bitmap_size: usize,
    total_pages: usize,
    free_pages: usize,
}

unsafe impl Send for BitmapAllocator {}
unsafe impl Sync for BitmapAllocator {}

impl BitmapAllocator {
    fn set_bit(&mut self, bit: usize) {
        let byte_idx = bit / 8;
        let bit_idx = bit % 8;
        unsafe {
            let byte = self.bitmap.add(byte_idx);
            *byte |= 1 << bit_idx;
        }
    }

    fn clear_bit(&mut self, bit: usize) {
        let byte_idx = bit / 8;
        let bit_idx = bit % 8;
        unsafe {
            let byte = self.bitmap.add(byte_idx);
            *byte &= !(1 << bit_idx);
        }
    }

    fn test_bit(&self, bit: usize) -> bool {
        let byte_idx = bit / 8;
        let bit_idx = bit % 8;
        unsafe {
            let byte = *self.bitmap.add(byte_idx);
            byte & (1 << bit_idx) != 0
        }
    }

    fn alloc_page(&mut self) -> Option<u64> {
        for i in 0..self.total_pages {
            if !self.test_bit(i) {
                self.set_bit(i);
                self.free_pages -= 1;
                return Some((i * PAGE_SIZE) as u64);
            }
        }
        None
    }

    fn free_page(&mut self, addr: u64) {
        let page = addr as usize / PAGE_SIZE;
        if page < self.total_pages && self.test_bit(page) {
            self.clear_bit(page);
            self.free_pages += 1;
        }
    }
}

pub fn init(memmap: &[&Entry], hhdm: VirtAddr) {
    let mut highest_addr: u64 = 0;
    for entry in memmap.iter() {
        let end = entry.base + entry.length;
        if end > highest_addr {
            highest_addr = end;
        }
    }

    let total_pages = (highest_addr as usize + PAGE_SIZE - 1) / PAGE_SIZE;
    let bitmap_size = (total_pages + 7) / 8;

    let mut bitmap_addr: Option<u64> = None;
    for entry in memmap.iter() {
        if entry.entry_type == EntryType::USABLE && entry.length >= bitmap_size as u64 {
            bitmap_addr = Some(entry.base);
            break;
        }
    }

    let bitmap_addr = bitmap_addr.expect("No space for PMM bitmap");
    let bitmap_ptr = (bitmap_addr + hhdm.as_u64()) as *mut u8;

    unsafe {
        core::ptr::write_bytes(bitmap_ptr, 0xff, bitmap_size);
    }

    let mut allocator = BitmapAllocator {
        bitmap: bitmap_ptr,
        bitmap_size,
        total_pages,
        free_pages: 0,
    };

    for entry in memmap.iter() {
        if entry.entry_type == EntryType::USABLE {
            let start_page = (entry.base as usize + PAGE_SIZE - 1) / PAGE_SIZE;
            let end_page = (entry.base + entry.length) as usize / PAGE_SIZE;

            for page in start_page..end_page {
                allocator.clear_bit(page);
                allocator.free_pages += 1;
            }
        }
    }

    let bitmap_start_page = bitmap_addr as usize / PAGE_SIZE;
    let bitmap_end_page = (bitmap_addr as usize + bitmap_size + PAGE_SIZE - 1) / PAGE_SIZE;

    for page in bitmap_start_page..bitmap_end_page {
        if !allocator.test_bit(page) {
            allocator.set_bit(page);
            allocator.free_pages -= 1;
        }
    }

    *PMM.lock() = Some(allocator);
}

pub fn alloc() -> Option<u64> {
    PMM.lock().as_mut()?.alloc_page()
}

pub fn free(addr: u64) {
    if let Some(pmm) = PMM.lock().as_mut() {
        pmm.free_page(addr);
    }
}

pub fn free_pages() -> usize {
    PMM.lock().as_ref().map(|p| p.free_pages).unwrap_or(0)
}
