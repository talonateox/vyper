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

pub fn create_user_page_table() -> Result<u64, &'static str> {
    let pml4_phys = pmm::alloc().ok_or("out of memory")?;

    unsafe {
        let pml4_virt = phys_to_virt(PhysAddr::new(pml4_phys));
        core::ptr::write_bytes(pml4_virt.as_mut_ptr::<u8>(), 0, 4096);
    }

    unsafe {
        let (current_pml4_frame, _) = Cr3::read();
        let current_pml4_virt = phys_to_virt(current_pml4_frame.start_address());
        let current_pml4: &PageTable = &*current_pml4_virt.as_ptr();

        let new_pml4_virt = phys_to_virt(PhysAddr::new(pml4_phys));
        let new_pml4: &mut PageTable = &mut *new_pml4_virt.as_mut_ptr();

        for i in 256..512 {
            new_pml4[i] = current_pml4[i].clone();
        }

        for i in 0..256 {
            if !current_pml4[i].is_unused() {
                let flags = current_pml4[i].flags();
                if flags.contains(PageTableFlags::PRESENT)
                    && !flags.contains(PageTableFlags::USER_ACCESSIBLE)
                {
                    new_pml4[i] = current_pml4[i].clone();
                }
            }
        }
    }

    Ok(pml4_phys)
}

pub unsafe fn switch_page_table(pml4_phys: u64) {
    let frame = PhysFrame::containing_address(PhysAddr::new(pml4_phys));
    Cr3::write(frame, x86_64::registers::control::Cr3Flags::empty());
}

pub unsafe fn free_page_table(pml4_phys: u64) {
    let pml4_virt = phys_to_virt(PhysAddr::new(pml4_phys));
    let pml4: &PageTable = &*pml4_virt.as_ptr();

    for pml4_idx in 0..256 {
        if !pml4[pml4_idx].is_unused() {
            free_pml4_entry(pml4, pml4_idx);
        }
    }

    pmm::free(pml4_phys);
}

unsafe fn free_pml4_entry(pml4: &PageTable, pml4_idx: usize) {
    let pdpt_phys = pml4[pml4_idx].addr();
    let pdpt_virt = phys_to_virt(pdpt_phys);
    let pdpt: &PageTable = &*pdpt_virt.as_ptr();

    for pdpt_idx in 0..512 {
        if !pdpt[pdpt_idx].is_unused() {
            free_pdpt_entry(pdpt, pdpt_idx);
        }
    }

    pmm::free(pdpt_phys.as_u64());
}

unsafe fn free_pdpt_entry(pdpt: &PageTable, pdpt_idx: usize) {
    let pd_phys = pdpt[pdpt_idx].addr();
    let pd_virt = phys_to_virt(pd_phys);
    let pd: &PageTable = &*pd_virt.as_ptr();

    for pd_idx in 0..512 {
        if !pd[pd_idx].is_unused() {
            free_pd_entry(pd, pd_idx);
        }
    }

    pmm::free(pd_phys.as_u64());
}

unsafe fn free_pd_entry(pd: &PageTable, pd_idx: usize) {
    let pt_phys = pd[pd_idx].addr();
    let pt_virt = phys_to_virt(pt_phys);
    let pt: &PageTable = &*pt_virt.as_ptr();

    for pt_idx in 0..512 {
        if !pt[pt_idx].is_unused() {
            let page_phys = pt[pt_idx].addr();
            pmm::free(page_phys.as_u64());
        }
    }

    pmm::free(pt_phys.as_u64());
}

pub fn map_page_in_table(
    pml4_phys: u64,
    virt: VirtAddr,
    phys: PhysAddr,
    flags: PageTableFlags,
) -> Result<(), &'static str> {
    unsafe {
        let pml4_virt = phys_to_virt(PhysAddr::new(pml4_phys));
        let pml4: &mut PageTable = &mut *pml4_virt.as_mut_ptr();
        let mut mapper = OffsetPageTable::new(pml4, VirtAddr::new(hhdm()));
        let mut allocator = PmmFrameAllocator;

        let page: Page<Size4KiB> = Page::containing_address(virt);
        let frame = PhysFrame::containing_address(phys);

        mapper
            .map_to(page, frame, flags, &mut allocator)
            .map_err(|_| "failed to map page")?
            .flush();
    }

    Ok(())
}
