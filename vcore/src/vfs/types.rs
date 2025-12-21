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

#[derive(Debug, Clone, Copy)]
pub struct OpenFlags {
    pub read: bool,
    pub write: bool,
    pub create: bool,
    pub truncate: bool,
    pub append: bool,
}

impl OpenFlags {
    pub const READ: Self = Self {
        read: true,
        write: false,
        create: false,
        truncate: false,
        append: false,
    };

    pub const WRITE: Self = Self {
        read: false,
        write: true,
        create: true,
        truncate: true,
        append: false,
    };

    pub const READ_WRITE: Self = Self {
        read: true,
        write: true,
        create: false,
        truncate: false,
        append: false,
    };

    pub const APPEND: Self = Self {
        read: false,
        write: true,
        create: true,
        truncate: false,
        append: true,
    };
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
