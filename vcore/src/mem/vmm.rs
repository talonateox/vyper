use spin::Mutex;
use x86_64::{
    PhysAddr, VirtAddr,
    registers::control::Cr3,
    structures::paging::{
        FrameAllocator, Mapper, OffsetPageTable, Page, PageTable, PageTableFlags, PhysFrame,
        Size4KiB, Translate,
    },
};

use crate::mem::pmm;

static HHDM_OFFSET: Mutex<Option<u64>> = Mutex::new(None);
static KERNEL_PML4_PHYS: Mutex<Option<PhysAddr>> = Mutex::new(None);

pub struct PmmFrameAllocator;

unsafe impl FrameAllocator<Size4KiB> for PmmFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        let addr = pmm::alloc()?;
        Some(PhysFrame::containing_address(PhysAddr::new(addr)))
    }
}

pub fn init(hhdm: VirtAddr) {
    *HHDM_OFFSET.lock() = Some(hhdm.as_u64());

    let (pml4_frame, _) = Cr3::read();
    *KERNEL_PML4_PHYS.lock() = Some(pml4_frame.start_address());
}

fn hhdm() -> u64 {
    HHDM_OFFSET.lock().expect("VMM not initialized")
}

pub fn phys_to_virt(phys: PhysAddr) -> VirtAddr {
    VirtAddr::new(phys.as_u64() + hhdm())
}

pub fn virt_to_phys(virt: VirtAddr) -> Option<PhysAddr> {
    let hhdm_val = hhdm();
    if virt.as_u64() >= hhdm_val {
        return Some(PhysAddr::new(virt.as_u64() - hhdm_val));
    }

    unsafe {
        let mapper = get_current_page_table();
        mapper.translate_addr(virt)
    }
}

unsafe fn get_current_page_table() -> OffsetPageTable<'static> {
    let (pml4_frame, _) = Cr3::read();
    let pml4_virt = phys_to_virt(pml4_frame.start_address());
    let pml4: &'static mut PageTable = unsafe { &mut *pml4_virt.as_mut_ptr() };
    unsafe { OffsetPageTable::new(pml4, VirtAddr::new(hhdm())) }
}

unsafe fn get_page_table_at(pml4_phys: PhysAddr) -> OffsetPageTable<'static> {
    let pml4_virt = phys_to_virt(pml4_phys);
    let pml4: &'static mut PageTable = unsafe { &mut *pml4_virt.as_mut_ptr() };
    unsafe { OffsetPageTable::new(pml4, VirtAddr::new(hhdm())) }
}

pub fn map_page(virt: VirtAddr, phys: PhysAddr, flags: PageTableFlags) -> Result<(), &'static str> {
    let page: Page<Size4KiB> = Page::containing_address(virt);
    let frame = PhysFrame::containing_address(phys);

    unsafe {
        let mut mapper = get_current_page_table();
        let mut allocator = PmmFrameAllocator;

        mapper
            .map_to(page, frame, flags, &mut allocator)
            .map_err(|_| "failed to map page")?
            .flush();
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
        let mut mapper = get_current_page_table();
        let (frame, flush) = mapper.unmap(page).map_err(|_| "failed to unmap page")?;
        flush.flush();
        Ok(frame.start_address())
    }
}

pub fn is_mapped(virt: VirtAddr) -> bool {
    unsafe {
        let mapper = get_current_page_table();
        mapper.translate_addr(virt).is_some()
    }
}

pub struct AddressSpace {
    pml4_phys: PhysAddr,
}

impl AddressSpace {
    pub fn new() -> Result<Self, &'static str> {
        let pml4_phys_addr = pmm::alloc().ok_or("out of memory for PML4")?;
        let pml4_phys = PhysAddr::new(pml4_phys_addr);

        unsafe {
            let pml4_virt = phys_to_virt(pml4_phys).as_mut_ptr::<u8>();
            core::ptr::write_bytes(pml4_virt, 0, 4096);
        }

        let kernel_pml4_phys = KERNEL_PML4_PHYS
            .lock()
            .ok_or("kernel PML4 not initialized")?;

        crate::info!(
            "new AS: pml4={:x} kernel_pml4={:x}",
            pml4_phys.as_u64(),
            kernel_pml4_phys.as_u64()
        );

        unsafe {
            let kernel_pml4 = phys_to_virt(kernel_pml4_phys).as_ptr::<u64>();
            let new_pml4 = phys_to_virt(pml4_phys).as_mut_ptr::<u64>();

            for i in 256..512 {
                let entry = kernel_pml4.add(i).read();
                new_pml4.add(i).write(entry);
            }
        }

        Ok(Self { pml4_phys })
    }

    pub fn cr3_value(&self) -> u64 {
        self.pml4_phys.as_u64()
    }

    pub fn map_page(
        &self,
        virt: VirtAddr,
        phys: PhysAddr,
        flags: PageTableFlags,
    ) -> Result<(), &'static str> {
        let page: Page<Size4KiB> = Page::containing_address(virt);
        let frame = PhysFrame::containing_address(phys);

        unsafe {
            let mut mapper = get_page_table_at(self.pml4_phys);
            let mut allocator = PmmFrameAllocator;

            mapper
                .map_to(page, frame, flags, &mut allocator)
                .map_err(|_| "failed to map page in address space")?
                .ignore();
        }

        Ok(())
    }

    pub fn map_page_alloc(
        &self,
        virt: VirtAddr,
        flags: PageTableFlags,
    ) -> Result<PhysAddr, &'static str> {
        let phys_addr = pmm::alloc().ok_or("out of memory")?;
        let phys = PhysAddr::new(phys_addr);

        unsafe {
            let virt_ptr = phys_to_virt(phys).as_mut_ptr::<u8>();
            core::ptr::write_bytes(virt_ptr, 0, 4096);
        }

        self.map_page(virt, phys, flags)?;
        Ok(phys)
    }

    pub fn is_mapped(&self, virt: VirtAddr) -> bool {
        unsafe {
            let mapper = get_page_table_at(self.pml4_phys);
            mapper.translate_addr(virt).is_some()
        }
    }

    pub fn write(&self, virt: VirtAddr, data: &[u8]) -> Result<(), &'static str> {
        let mut offset = 0;

        while offset < data.len() {
            let current_virt = VirtAddr::new(virt.as_u64() + offset as u64);
            let page_offset = (current_virt.as_u64() & 0xFFF) as usize;
            let bytes_in_page = core::cmp::min(4096 - page_offset, data.len() - offset);

            let phys = unsafe {
                let mapper = get_page_table_at(self.pml4_phys);
                mapper
                    .translate_addr(current_virt)
                    .ok_or("page not mapped")?
            };

            unsafe {
                let dest = phys_to_virt(phys).as_mut_ptr::<u8>();
                core::ptr::copy_nonoverlapping(data.as_ptr().add(offset), dest, bytes_in_page);
            }

            offset += bytes_in_page;
        }

        Ok(())
    }

    pub fn zero(&self, virt: VirtAddr, len: usize) -> Result<(), &'static str> {
        let mut offset = 0;

        while offset < len {
            let current_virt = VirtAddr::new(virt.as_u64() + offset as u64);
            let page_offset = (current_virt.as_u64() & 0xFFF) as usize;
            let bytes_in_page = core::cmp::min(4096 - page_offset, len - offset);

            let phys = unsafe {
                let mapper = get_page_table_at(self.pml4_phys);
                mapper
                    .translate_addr(current_virt)
                    .ok_or("page not mapped")?
            };

            unsafe {
                let dest = phys_to_virt(phys).as_mut_ptr::<u8>();
                core::ptr::write_bytes(dest, 0, bytes_in_page);
            }

            offset += bytes_in_page;
        }

        Ok(())
    }
}

impl Drop for AddressSpace {
    fn drop(&mut self) {
        // this leaks for now, ill do this later i guess
    }
}
