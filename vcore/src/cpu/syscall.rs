use core::arch::{asm, naked_asm};

use x86_64::{
    VirtAddr,
    registers::{
        control::{Efer, EferFlags},
        model_specific::{KernelGsBase, LStar, SFMask, Star},
        rflags::RFlags,
    },
};

use crate::{cpu::gdt, drivers::keyboard, error, info, print, sched, vfs};

#[repr(C, align(16))]
struct CpuLocal {
    user_rsp: u64,
    kernel_rsp: u64,
}

static mut CPU_LOCAL: CpuLocal = CpuLocal {
    user_rsp: 0,
    kernel_rsp: 0,
};

static mut SYSCALL_STACK: [u8; 4096 * 4] = [0; 4096 * 4];

pub fn init() {
    let selectors = gdt::selectors();

    unsafe {
        Star::write(
            selectors.user_code,
            selectors.user_data,
            selectors.kernel_code,
            selectors.kernel_data,
        )
        .expect("failed to set STAR");

        LStar::write(VirtAddr::new(syscall_entry as *const () as u64));
        SFMask::write(RFlags::empty());

        let efer = Efer::read();
        Efer::write(efer | EferFlags::SYSTEM_CALL_EXTENSIONS);

        let cpu_local_addr = &CPU_LOCAL as *const _ as u64;
        CPU_LOCAL.kernel_rsp = SYSCALL_STACK.as_ptr() as u64 + SYSCALL_STACK.len() as u64;

        KernelGsBase::write(VirtAddr::new(cpu_local_addr));
    }
}

#[unsafe(naked)]
unsafe extern "C" fn syscall_entry() {
    naked_asm!(
        "swapgs",
        "mov gs:[0], rsp",
        "mov rsp, gs:[8]",

        "push rcx",
        "push r11",
        "push rax",

        "push rdi",
        "push rsi",
        "push rdx",
        "push r10",
        "push r8",
        "push r9",
        "push rbx",
        "push rbp",
        "push r12",
        "push r13",
        "push r14",
        "push r15",

        "mov rdi, rax",
        "mov rsi, [rsp + 11*8]",
        "mov rdx, [rsp + 10*8]",
        "mov rcx, [rsp + 9*8]",
        "mov r8, [rsp + 8*8]",
        "mov r9, [rsp + 7*8]",

        "call {handler}",

        "pop r15",
        "pop r14",
        "pop r13",
        "pop r12",
        "pop rbp",
        "pop rbx",
        "pop r9",
        "pop r8",
        "pop r10",
        "pop rdx",
        "pop rsi",
        "pop rdi",

        "add rsp, 8",
        "pop r11",
        "pop rcx",

        "mov rsp, gs:[0]",
        "swapgs",

        "sysretq",
        handler = sym syscall_handler,
    );
}

pub const SYS_EXIT: u64 = 0;
pub const SYS_WRITE: u64 = 1;
pub const SYS_READ: u64 = 2;
pub const SYS_OPEN: u64 = 3;
pub const SYS_CLOSE: u64 = 4;
pub const SYS_GETDENTS: u64 = 5;
pub const SYS_MKDIR: u64 = 6;
pub const SYS_UNLINK: u64 = 7;
pub const SYS_RMDIR: u64 = 8;
pub const SYS_CHDIR: u64 = 9;
pub const SYS_GETCWD: u64 = 10;

extern "C" fn syscall_handler(
    num: u64,
    arg1: u64,
    arg2: u64,
    arg3: u64,
    _arg4: u64,
    _arg5: u64,
) -> u64 {
    match num {
        SYS_EXIT => {
            info!("task exited with code {}", arg1);
            sched::exit();
            0
        }

        SYS_WRITE => {
            let fd = arg1 as usize;
            let buf = arg2 as *const u8;
            let len = arg3 as usize;

            if fd == 1 || fd == 2 {
                for i in 0..len {
                    let c = unsafe { *buf.add(i) };
                    print!("{}", c as char);
                }
                return len as u64;
            }

            let result = sched::with_fd_table(|table| match table.get_mut(fd)? {
                vfs::FdKind::File(handle) => {
                    let slice = unsafe { core::slice::from_raw_parts(buf, len) };
                    handle.write(slice)
                }
                vfs::FdKind::Stdout | vfs::FdKind::Stderr => {
                    for i in 0..len {
                        let c = unsafe { *buf.add(i) };
                        print!("{}", c as char);
                    }
                    Ok(len)
                }
                _ => Err(vfs::VfsError::PermissionDenied),
            });

            result.unwrap_or(0) as u64
        }

        SYS_READ => {
            let fd = arg1 as usize;
            let buf = arg2 as *mut u8;
            let len = arg3 as usize;

            if fd == 0 {
                while !keyboard::has_input() {
                    x86_64::instructions::hlt();
                }

                let mut count = 0;
                while count < len {
                    if let Some(c) = keyboard::read_char() {
                        unsafe { *buf.add(count) = c };
                        count += 1;
                        if c == b'\n' {
                            break;
                        }
                    } else {
                        break;
                    }
                }
                return count as u64;
            }

            let result = sched::with_fd_table(|table| match table.get_mut(fd)? {
                vfs::FdKind::File(handle) => {
                    let slice = unsafe { core::slice::from_raw_parts_mut(buf, len) };
                    handle.read(slice)
                }
                vfs::FdKind::Stdin => {
                    while !keyboard::has_input() {
                        x86_64::instructions::hlt();
                    }
                    let mut count = 0;
                    while count < len {
                        if let Some(c) = keyboard::read_char() {
                            unsafe { *buf.add(count) = c };
                            count += 1;
                            if c == b'\n' {
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                    Ok(count)
                }
                _ => Err(vfs::VfsError::PermissionDenied),
            });

            result.unwrap_or(0) as u64
        }

        SYS_OPEN => {
            let path_ptr = arg1 as *const u8;
            let path_len = arg2 as usize;
            let flags = arg3 as u32;

            let path = unsafe {
                let slice = core::slice::from_raw_parts(path_ptr, path_len);
                core::str::from_utf8_unchecked(slice)
            };

            let cwd = sched::get_cwd().unwrap_or_else(|| "/".into());
            let path = vfs::resolve_path(path, &cwd);

            let open_flags = vfs::OpenFlags::from_bits(flags);

            if open_flags.contains(vfs::OpenFlags::O_DIRECTORY) {
                match vfs::readdir(&path) {
                    Ok(entries) => {
                        let result = sched::with_fd_table(|table| {
                            table.alloc(vfs::FdKind::Directory {
                                path: path.into(),
                                entries,
                                position: 0,
                            })
                        });
                        result.map(|fd| fd as u64).unwrap_or(u64::MAX)
                    }
                    Err(_) => u64::MAX,
                }
            } else {
                match vfs::open(&path, open_flags) {
                    Ok(handle) => {
                        let result =
                            sched::with_fd_table(|table| table.alloc(vfs::FdKind::File(handle)));
                        result.map(|fd| fd as u64).unwrap_or(u64::MAX)
                    }
                    Err(_) => u64::MAX,
                }
            }
        }

        SYS_CLOSE => {
            let fd = arg1 as usize;
            let result = sched::with_fd_table(|table| table.close(fd));
            if result.is_ok() { 0 } else { u64::MAX }
        }

        SYS_GETDENTS => {
            let fd = arg1 as usize;
            let buf_ptr = arg2 as *mut u8;
            let buf_len = arg3 as usize;

            let result = sched::with_fd_table(|table| match table.get_mut(fd)? {
                vfs::FdKind::Directory {
                    entries, position, ..
                } => {
                    let mut offset = 0usize;

                    while *position < entries.len() {
                        let entry = &entries[*position];
                        let name_bytes = entry.name.as_bytes();
                        let entry_size = 1 + 2 + name_bytes.len();

                        if offset + entry_size > buf_len {
                            break;
                        }

                        unsafe {
                            let file_type: u8 = match entry.file_type {
                                vfs::FileType::File => 1,
                                vfs::FileType::Directory => 2,
                                vfs::FileType::Device => 3,
                            };
                            *buf_ptr.add(offset) = file_type;
                            offset += 1;

                            let name_len = name_bytes.len() as u16;
                            *buf_ptr.add(offset) = (name_len & 0xFF) as u8;
                            *buf_ptr.add(offset + 1) = (name_len >> 8) as u8;
                            offset += 2;

                            core::ptr::copy_nonoverlapping(
                                name_bytes.as_ptr(),
                                buf_ptr.add(offset),
                                name_bytes.len(),
                            );
                            offset += name_bytes.len();
                        }

                        *position += 1;
                    }

                    Ok(offset)
                }
                _ => Err(vfs::VfsError::NotADirectory),
            });

            result.unwrap_or(0) as u64
        }

        SYS_MKDIR => {
            let path_ptr = arg1 as *const u8;
            let path_len = arg2 as usize;

            let path = unsafe {
                let slice = core::slice::from_raw_parts(path_ptr, path_len);
                core::str::from_utf8_unchecked(slice)
            };

            let cwd = sched::get_cwd().unwrap_or_else(|| "/".into());
            let path = vfs::resolve_path(path, &cwd);

            match vfs::mkdir(&path) {
                Ok(()) => 0,
                Err(_) => u64::MAX,
            }
        }

        SYS_UNLINK => {
            let path_ptr = arg1 as *const u8;
            let path_len = arg2 as usize;

            let path = unsafe {
                let slice = core::slice::from_raw_parts(path_ptr, path_len);
                core::str::from_utf8_unchecked(slice)
            };

            let cwd = sched::get_cwd().unwrap_or_else(|| "/".into());
            let path = vfs::resolve_path(path, &cwd);

            match vfs::remove(&path) {
                Ok(()) => 0,
                Err(_) => u64::MAX,
            }
        }

        SYS_RMDIR => {
            let path_ptr = arg1 as *const u8;
            let path_len = arg2 as usize;

            let path = unsafe {
                let slice = core::slice::from_raw_parts(path_ptr, path_len);
                core::str::from_utf8_unchecked(slice)
            };

            let cwd = sched::get_cwd().unwrap_or_else(|| "/".into());
            let path = vfs::resolve_path(path, &cwd);

            match vfs::rmdir(&path) {
                Ok(()) => 0,
                Err(_) => u64::MAX,
            }
        }

        SYS_CHDIR => {
            let path_ptr = arg1 as *const u8;
            let path_len = arg2 as usize;

            let path = unsafe {
                let slice = core::slice::from_raw_parts(path_ptr, path_len);
                core::str::from_utf8_unchecked(slice)
            };

            let cwd = sched::get_cwd().unwrap_or_else(|| "/".into());
            let path = vfs::resolve_path(path, &cwd);

            match vfs::metadata(&path) {
                Ok(meta) if meta.file_type == vfs::FileType::Directory => {
                    if sched::set_cwd(path).is_ok() {
                        0
                    } else {
                        u64::MAX
                    }
                }
                _ => u64::MAX,
            }
        }

        SYS_GETCWD => {
            let buf_ptr = arg1 as *mut u8;
            let buf_len = arg2 as usize;

            match sched::get_cwd() {
                Some(cwd) => {
                    let bytes = cwd.as_bytes();
                    let copy_len = bytes.len().min(buf_len);
                    unsafe {
                        core::ptr::copy_nonoverlapping(bytes.as_ptr(), buf_ptr, copy_len);
                    }
                    copy_len as u64
                }
                None => u64::MAX,
            }
        }

        _ => {
            error!("unknown syscall: {}", num);
            u64::MAX
        }
    }
}

pub unsafe fn jump_to_usermode(entry: u64, user_stack: u64) -> ! {
    let selectors = gdt::selectors();

    let user_cs = selectors.user_code.0 as u64;
    let user_ss = selectors.user_data.0 as u64;

    unsafe {
        asm!(
            "push {user_ss}",
            "push {user_stack}",
            "push 0x202",
            "push {user_cs}",
            "push {entry}",
            "iretq",
            user_ss = in(reg) user_ss,
            user_stack = in(reg) user_stack,
            user_cs = in(reg) user_cs,
            entry = in(reg) entry,
            options(noreturn)
        )
    }
}
