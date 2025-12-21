#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![allow(dead_code)]
#![allow(rust_2024_compatibility)]

extern crate alloc;

mod cpu;
mod drivers;
mod elf;
mod fb;
mod font;
mod mem;
mod sched;
mod vfs;

use core::arch::asm;

use alloc::boxed::Box;
use limine::BaseRevision;
use limine::request::{
    FramebufferRequest, HhdmRequest, MemoryMapRequest, RequestsEndMarker, RequestsStartMarker,
};
use x86_64::VirtAddr;

use crate::fb::{Framebuffer, terminal};
use crate::vfs::TasksFs;

#[used]
#[unsafe(link_section = ".requests")]
static BASE_REVISION: BaseRevision = BaseRevision::new();

#[used]
#[unsafe(link_section = ".requests")]
static FRAMEBUFFER_REQUEST: FramebufferRequest = FramebufferRequest::new();

#[used]
#[unsafe(link_section = ".requests")]
static HHDM_REQUEST: HhdmRequest = HhdmRequest::new();

#[used]
#[unsafe(link_section = ".requests")]
static MEMORY_MAP_REQUEST: MemoryMapRequest = MemoryMapRequest::new();

#[used]
#[unsafe(link_section = ".requests_start_marker")]
static _START_MARKER: RequestsStartMarker = RequestsStartMarker::new();
#[used]
#[unsafe(link_section = ".requests_end_marker")]
static _END_MARKER: RequestsEndMarker = RequestsEndMarker::new();

static INIT_ELF: &[u8] = include_bytes!("../../target/x86_64-unknown-none/release/shell");

fn setup_fs() {
    vfs::mount("/", Box::new(vfs::TmpFs::new())).expect("failed to mount root");
    vfs::mkdir("/system").unwrap();
    vfs::mkdir("/system/cmd").unwrap();
    vfs::mkdir("/system/lib").unwrap();
    vfs::mkdir("/live").unwrap();

    vfs::mkdir("/live/tasks").unwrap();
    vfs::mount("/live/tasks", Box::new(TasksFs::new())).expect("failed to mount tasksfs");
}

#[unsafe(no_mangle)]
unsafe extern "C" fn kmain() -> ! {
    assert!(BASE_REVISION.is_supported());

    let hhdm = VirtAddr::new(HHDM_REQUEST.get_response().unwrap().offset());

    let framebuffer = Framebuffer::from_limine(&FRAMEBUFFER_REQUEST);
    terminal::init(framebuffer, &font::FONT);

    terminal::set_fg(0x00ff7f);
    println!("      _                   .-=-.          .-==-.");
    println!("     {{ }}      __        .' O o '.       /  -<' )");
    println!("     {{ }}    .' O'.     / o .-. O \\     /  .--v`");
    println!("     {{ }}   / .-. o\\   /O  /   \\  o\\   /O /");
    println!("      \\ `-` /   \\ O`-'o  /     \\  O`-`o /");
    println!("       `-.-`     '.____.'       `.____.'\n");
    terminal::set_fg(0xffffff);

    info!("Beginning BOOT");
    cpu::init();

    let mmap = MEMORY_MAP_REQUEST
        .get_response()
        .expect("no memmap")
        .entries();

    mem::pmm::init(mmap, hhdm);
    info!("PMM {}MB free", mem::pmm::free_pages() * 4096 / 1024 / 1024);

    mem::vmm::init(hhdm);
    info!("VMM loaded");

    mem::heap::init().expect("heap init failed");
    info!("HEAP {}KB", mem::heap::size());

    cpu::apic::init();
    info!("APIC loaded");

    setup_fs();

    sched::init();

    sched::spawn_elf(INIT_ELF).expect("failed to spawn init");

    x86_64::instructions::interrupts::enable();

    hcf();
}

#[panic_handler]
fn rust_panic(info: &core::panic::PanicInfo) -> ! {
    error!("{}", info.message());
    error!("at {}", info.location().unwrap());
    hcf()
}

fn hcf() -> ! {
    loop {
        unsafe {
            #[cfg(target_arch = "x86_64")]
            asm!("hlt");
            #[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
            asm!("wfi");
            #[cfg(target_arch = "loongarch64")]
            asm!("idle 0");
        }
    }
}
