use spin::Mutex;
use x86_64::{
    PhysAddr, VirtAddr,
    registers::control::Cr3,
    structures::paging::{
        FrameAllocator, Mapper, OffsetPageTable, Page, PageTable, PageTableFlags, PhysFrame,
        Size4KiB,
    },
};

use crate::mem::pmm;

static HHDM_OFFSET: Mutex<Option<u64>> = Mutex::new(None);

pub struct PmmFrameAllocator;

unsafe impl FrameAllocator<Size4KiB> for PmmFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        let addr = pmm::alloc()?;
        Some(PhysFrame::containing_address(PhysAddr::new(addr)))
    }
}

pub fn init(hhdm: VirtAddr) {
    *HHDM_OFFSET.lock() = Some(hhdm.as_u64());
}

fn hhdm() -> u64 {
    HHDM_OFFSET.lock().expect("VMM not initialized")
}

pub fn phys_to_virt(phys: PhysAddr) -> VirtAddr {
    VirtAddr::new(phys.as_u64() + hhdm())
}

unsafe fn get_page_table() -> OffsetPageTable<'static> {
    let (pml4_frame, _) = Cr3::read();
    let pml4_virt = phys_to_virt(pml4_frame.start_address());
    let pml4: &'static mut PageTable = unsafe { &mut *pml4_virt.as_mut_ptr() };
    unsafe { OffsetPageTable::new(pml4, VirtAddr::new(hhdm())) }
}

pub fn map_page(virt: VirtAddr, phys: PhysAddr, flags: PageTableFlags) -> Result<(), &'static str> {
    let page: Page<Size4KiB> = Page::containing_address(virt);
    let frame = PhysFrame::containing_address(phys);

    unsafe {
        let mut mapper = get_page_table();
        let mut allocator = PmmFrameAllocator;

        mapper
            .map_to(page, frame, flags, &mut allocator)
            .map_err(|_| "failed to map page")?
            .flush()
    }

    Ok(())
}

pub fn map_page_alloc(virt: VirtAddr, flags: PageTableFlags) -> Result<PhysAddr, &'static str> {
    let phys_addr = pmm::alloc().ok_or("out of memory")?;
    let phys = PhysAddr::new(phys_addr);

    unsafe {
        let virt_ptr = phys_to_virt(phys).as_mut_ptr::<u8>();
        core::ptr::write_bytes(virt_ptr, 0, 4096);
    }

    map_page(virt, phys, flags)?;
    Ok(phys)
}

pub fn unmap_page(virt: VirtAddr) -> Result<PhysAddr, &'static str> {
    let page: Page<Size4KiB> = Page::containing_address(virt);

    unsafe {
        let mut mapper = get_page_table();
        let (frame, flush) = mapper.unmap(page).map_err(|_| "failed to unmap page")?;
        flush.flush();
        Ok(frame.start_address())
    }
}

pub fn is_mapped(virt: VirtAddr) -> bool {
    use x86_64::structures::paging::Translate;

    unsafe {
        let mapper = get_page_table();
        mapper.translate_addr(virt).is_some()
    }
}
