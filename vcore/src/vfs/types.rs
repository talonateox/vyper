use alloc::{boxed::Box, string::String, vec::Vec};

pub const MAX_FDS: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FileType {
    File,
    Directory,
    Device,
}

#[derive(Debug, Clone)]
pub struct Metadata {
    pub file_type: FileType,
    pub size: usize,
}

#[derive(Debug, Clone)]
pub struct DirEntry {
    pub name: String,
    pub file_type: FileType,
}

#[derive(Debug, Clone, Copy)]
pub enum SeekFrom {
    Start(usize),
    Current(isize),
    End(isize),
}

#[derive(Debug, Clone, Copy, Default)]
pub struct OpenFlags(u32);

impl OpenFlags {
    pub const O_RDONLY: Self = Self(0);
    pub const O_WRONLY: Self = Self(1);
    pub const O_RDWR: Self = Self(2);

    pub const O_CREAT: Self = Self(0o100);
    pub const O_EXCL: Self = Self(0o200);
    pub const O_TRUNC: Self = Self(0o1000);
    pub const O_APPEND: Self = Self(0o2000);
    pub const O_DIRECTORY: Self = Self(0o200000);

    pub const fn empty() -> Self {
        Self(0)
    }

    pub const fn bits(&self) -> u32 {
        self.0
    }

    pub const fn from_bits(bits: u32) -> Self {
        Self(bits)
    }

    pub const fn contains(&self, other: Self) -> bool {
        if other.0 == 0 {
            return (self.0 & 3) == 0;
        }
        (self.0 & other.0) == other.0
    }

    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    pub const fn access_mode(&self) -> u32 {
        self.0 & 3
    }

    pub const fn is_readable(&self) -> bool {
        let mode = self.0 & 3;
        mode == 0 || mode == 2
    }

    pub const fn is_writable(&self) -> bool {
        let mode = self.0 & 3;
        mode == 1 || mode == 2
    }

    pub const READ: Self = Self::O_RDONLY;
    pub const WRITE: Self = Self(1 | 0o100 | 0o1000);
    pub const READ_WRITE: Self = Self::O_RDWR;
    pub const APPEND: Self = Self(1 | 0o100 | 0o2000);
}

impl core::ops::BitOr for OpenFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl core::ops::BitOrAssign for OpenFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VfsError {
    NotFound,
    AlreadyExists,
    NotADirectory,
    IsADirectory,
    NotEmpty,
    InvalidPath,
    PermissionDenied,
    NoSpace,
    InvalidFd,
    NotSupported,
    IoError,
}

pub type VfsResult<T> = Result<T, VfsError>;

pub trait FileHandle: Send + Sync {
    fn read(&mut self, buf: &mut [u8]) -> VfsResult<usize>;
    fn write(&mut self, buf: &[u8]) -> VfsResult<usize>;
    fn seek(&mut self, pos: SeekFrom) -> VfsResult<usize>;
    fn metadata(&self) -> VfsResult<Metadata>;
}

pub trait Filesystem: Send + Sync {
    fn open(&self, path: &str, flags: OpenFlags) -> VfsResult<Box<dyn FileHandle>>;
    fn mkdir(&self, path: &str) -> VfsResult<()>;
    fn remove(&self, path: &str) -> VfsResult<()>;
    fn rmdir(&self, path: &str) -> VfsResult<()>;
    fn readdir(&self, path: &str) -> VfsResult<Vec<DirEntry>>;
    fn metadata(&self, path: &str) -> VfsResult<Metadata>;
    fn exists(&self, path: &str) -> bool {
        self.metadata(path).is_ok()
    }
}
