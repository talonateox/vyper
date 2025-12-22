use core::arch::asm;

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

pub const O_RDONLY: u64 = 0;
pub const O_WRONLY: u64 = 1;
pub const O_RDWR: u64 = 2;
pub const O_CREAT: u64 = 64;
pub const O_TRUNC: u64 = 512;
pub const O_APPEND: u64 = 1024;
pub const O_DIRECTORY: u64 = 65536;

pub fn exit(code: u64) -> ! {
    syscall1(SYS_EXIT, code);
    unreachable!()
}

pub fn write(fd: u64, buf: &[u8]) -> u64 {
    syscall3(SYS_WRITE, fd, buf.as_ptr() as u64, buf.len() as u64)
}

pub fn read(fd: u64, buf: &mut [u8]) -> u64 {
    syscall3(SYS_READ, fd, buf.as_mut_ptr() as u64, buf.len() as u64)
}

pub fn getch() -> u8 {
    let mut buf = [0u8; 1];
    read(0, &mut buf);
    buf[0]
}

pub fn open(path: &[u8], flags: u64) -> i64 {
    let result = syscall3(SYS_OPEN, path.as_ptr() as u64, path.len() as u64, flags);
    if result == u64::MAX {
        -1
    } else {
        result as i64
    }
}

pub fn close(fd: u64) -> i64 {
    let result = syscall1(SYS_CLOSE, fd);
    if result == u64::MAX { -1 } else { 0 }
}

pub fn touch(path: &[u8]) -> i64 {
    let fd = open(path, O_CREAT);
    if fd >= 0 {
        close(fd as u64);
        0
    } else {
        -1
    }
}

pub fn unlink(path: &[u8]) -> i64 {
    let result = syscall2(SYS_UNLINK, path.as_ptr() as u64, path.len() as u64);
    result as i64
}

pub fn mkdir(path: &[u8]) -> i64 {
    let result = syscall2(SYS_MKDIR, path.as_ptr() as u64, path.len() as u64);
    result as i64
}

pub fn rmdir(path: &[u8]) -> i64 {
    let result = syscall2(SYS_RMDIR, path.as_ptr() as u64, path.len() as u64);
    result as i64
}

pub fn chdir(path: &[u8]) -> i64 {
    let result = syscall2(SYS_CHDIR, path.as_ptr() as u64, path.len() as u64);
    if result == u64::MAX { -1 } else { 0 }
}

pub fn getcwd(buf: &mut [u8]) -> i64 {
    let result = syscall2(SYS_GETCWD, buf.as_mut_ptr() as u64, buf.len() as u64);
    if result == u64::MAX {
        -1
    } else {
        result as i64
    }
}

pub fn getdents(fd: u64, buf: &mut [u8]) -> i64 {
    let result = syscall3(SYS_GETDENTS, fd, buf.as_mut_ptr() as u64, buf.len() as u64);
    result as i64
}

#[derive(Clone)]
pub struct DirEntry {
    pub file_type: u8,
    pub name_len: usize,
    pub name: [u8; 256],
}

impl DirEntry {
    pub const fn empty() -> Self {
        Self {
            file_type: 0,
            name_len: 0,
            name: [0; 256],
        }
    }

    pub fn name(&self) -> &[u8] {
        &self.name[..self.name_len]
    }

    pub fn is_dir(&self) -> bool {
        self.file_type == 2
    }

    pub fn is_file(&self) -> bool {
        self.file_type == 1
    }
}

pub struct DirEntryIter<'a> {
    buf: &'a [u8],
    offset: usize,
}

impl<'a> DirEntryIter<'a> {
    pub fn new(buf: &'a [u8], len: usize) -> Self {
        Self {
            buf: &buf[..len],
            offset: 0,
        }
    }
}

impl<'a> Iterator for DirEntryIter<'a> {
    type Item = DirEntry;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset + 3 > self.buf.len() {
            return None;
        }

        let file_type = self.buf[self.offset];
        self.offset += 1;

        let name_len =
            (self.buf[self.offset] as usize) | ((self.buf[self.offset + 1] as usize) << 8);
        self.offset += 2;

        if self.offset + name_len > self.buf.len() || name_len > 256 {
            return None;
        }

        let mut entry = DirEntry::empty();
        entry.file_type = file_type;
        entry.name_len = name_len;
        entry.name[..name_len].copy_from_slice(&self.buf[self.offset..self.offset + name_len]);
        self.offset += name_len;

        Some(entry)
    }
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
