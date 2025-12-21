use alloc::boxed::Box;

use crate::vfs::{DirEntry, FileHandle, VfsError, VfsResult};

pub const MAX_FDS: usize = 64;

pub enum FdKind {
    File(Box<dyn FileHandle>),
    Directory {
        path: alloc::string::String,
        entries: alloc::vec::Vec<DirEntry>,
        position: usize,
    },
    Stdin,
    Stdout,
    Stderr,
}

pub struct FdTable {
    fds: [Option<FdKind>; MAX_FDS],
}

impl FdTable {
    pub fn new() -> Self {
        let mut table = Self {
            fds: core::array::from_fn(|_| None),
        };
        table.fds[0] = Some(FdKind::Stdin);
        table.fds[1] = Some(FdKind::Stdout);
        table.fds[2] = Some(FdKind::Stderr);
        table
    }

    pub fn alloc(&mut self, kind: FdKind) -> VfsResult<usize> {
        for i in 0..MAX_FDS {
            if self.fds[i].is_none() {
                self.fds[i] = Some(kind);
                return Ok(i);
            }
        }
        Err(VfsError::NoSpace)
    }

    pub fn get(&self, fd: usize) -> VfsResult<&FdKind> {
        if fd >= MAX_FDS {
            return Err(VfsError::InvalidFd);
        }
        self.fds[fd].as_ref().ok_or(VfsError::InvalidFd)
    }

    pub fn get_mut(&mut self, fd: usize) -> VfsResult<&mut FdKind> {
        if fd >= MAX_FDS {
            return Err(VfsError::InvalidFd);
        }
        self.fds[fd].as_mut().ok_or(VfsError::InvalidFd)
    }

    pub fn close(&mut self, fd: usize) -> VfsResult<()> {
        if fd >= MAX_FDS {
            return Err(VfsError::InvalidFd);
        }
        if fd < 3 {
            return Err(VfsError::PermissionDenied);
        }
        if self.fds[fd].is_none() {
            return Err(VfsError::InvalidFd);
        }
        self.fds[fd] = None;
        Ok(())
    }
}
