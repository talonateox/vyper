use core::arch::asm;

pub const SYS_EXIT: u64 = 0;
pub const SYS_WRITE: u64 = 1;

pub fn exit(code: u64) -> ! {
    syscall1(SYS_EXIT, code);
    unreachable!()
}

pub fn write(fd: u64, buf: &[u8]) -> u64 {
    syscall3(SYS_WRITE, fd, buf.as_ptr() as u64, buf.len() as u64)
}

#[inline(always)]
pub fn syscall0(num: u64) -> u64 {
    let ret: u64;
    unsafe {
        asm!(
            "syscall",
            in("rax") num,
            lateout("rax") ret,
            lateout("rcx") _,
            lateout("r11") _,
            options(nostack)
        );
    }
    ret
}

#[inline(always)]
pub fn syscall1(num: u64, arg1: u64) -> u64 {
    let ret: u64;
    unsafe {
        asm!(
            "syscall",
            in("rax") num,
            in("rdi") arg1,
            lateout("rax") ret,
            lateout("rcx") _,
            lateout("r11") _,
            options(nostack)
        );
    }
    ret
}

#[inline(always)]
pub fn syscall2(num: u64, arg1: u64, arg2: u64) -> u64 {
    let ret: u64;
    unsafe {
        asm!(
            "syscall",
            in("rax") num,
            in("rdi") arg1,
            in("rsi") arg2,
            lateout("rax") ret,
            lateout("rcx") _,
            lateout("r11") _,
            options(nostack)
        );
    }
    ret
}

#[inline(always)]
pub fn syscall3(num: u64, arg1: u64, arg2: u64, arg3: u64) -> u64 {
    let ret: u64;
    unsafe {
        asm!(
            "syscall",
            in("rax") num,
            in("rdi") arg1,
            in("rsi") arg2,
            in("rdx") arg3,
            lateout("rax") ret,
            lateout("rcx") _,
            lateout("r11") _,
            options(nostack)
        );
    }
    ret
}

#[inline(always)]
pub fn syscall4(num: u64, arg1: u64, arg2: u64, arg3: u64, arg4: u64) -> u64 {
    let ret: u64;
    unsafe {
        asm!(
            "syscall",
            in("rax") num,
            in("rdi") arg1,
            in("rsi") arg2,
            in("rdx") arg3,
            in("r10") arg4,
            lateout("rax") ret,
            lateout("rcx") _,
            lateout("r11") _,
            options(nostack)
        );
    }
    ret
}

#[inline(always)]
pub fn syscall5(num: u64, arg1: u64, arg2: u64, arg3: u64, arg4: u64, arg5: u64) -> u64 {
    let ret: u64;
    unsafe {
        asm!(
            "syscall",
            in("rax") num,
            in("rdi") arg1,
            in("rsi") arg2,
            in("rdx") arg3,
            in("r10") arg4,
            in("r8") arg5,
            lateout("rax") ret,
            lateout("rcx") _,
            lateout("r11") _,
            options(nostack)
        );
    }
    ret
}
