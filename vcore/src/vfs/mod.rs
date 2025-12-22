use alloc::{
    boxed::Box,
    string::{String, ToString},
    vec::Vec,
};
use spin::{Lazy, Mutex};

pub mod blockdev;
pub mod fd;
pub mod memfs;
pub mod tasksfs;
pub mod tmpfs;
pub mod types;

pub use blockdev::{BlockDevice, DevFs, ata::AtaBlockDevice};
pub use fd::FdKind;
pub use memfs::MemFs;
pub use tasksfs::TasksFs;
pub use tmpfs::TmpFs;
pub use types::*;

struct Mount {
    path: String,
    fs: Box<dyn Filesystem>,
}

pub struct Vfs {
    mounts: Vec<Mount>,
}

impl Vfs {
    pub fn new() -> Self {
        Self { mounts: Vec::new() }
    }

    pub fn mount(&mut self, path: &str, fs: Box<dyn Filesystem>) -> VfsResult<()> {
        let path = normalize_path(path);

        if self.mounts.iter().any(|m| m.path == path) {
            return Err(VfsError::AlreadyExists);
        }

        self.mounts.push(Mount { path, fs });
        self.mounts.sort_by(|a, b| b.path.len().cmp(&a.path.len()));

        Ok(())
    }

    pub fn unmount(&mut self, path: &str) -> VfsResult<()> {
        let path = normalize_path(path);
        let idx = self
            .mounts
            .iter()
            .position(|m| m.path == path)
            .ok_or(VfsError::NotFound)?;
        self.mounts.remove(idx);
        Ok(())
    }

    fn resolve(&self, path: &str) -> VfsResult<(&dyn Filesystem, String)> {
        let path = normalize_path(path);

        for mount in &self.mounts {
            if mount.path == "/" {
                return Ok((mount.fs.as_ref(), path.clone()));
            }
            if path == mount.path {
                return Ok((mount.fs.as_ref(), "/".to_string()));
            }
            if path.starts_with(&mount.path) {
                let rest = &path[mount.path.len()..];
                let relative = if rest.is_empty() || rest == "/" {
                    "/".to_string()
                } else if rest.starts_with('/') {
                    rest.to_string()
                } else {
                    continue;
                };
                return Ok((mount.fs.as_ref(), relative));
            }
        }

        Err(VfsError::NotFound)
    }

    pub fn open(&self, path: &str, flags: OpenFlags) -> VfsResult<Box<dyn FileHandle>> {
        let (fs, relative) = self.resolve(path)?;
        fs.open(&relative, flags)
    }

    pub fn mkdir(&self, path: &str) -> VfsResult<()> {
        let (fs, relative) = self.resolve(path)?;
        fs.mkdir(&relative)
    }

    pub fn remove(&self, path: &str) -> VfsResult<()> {
        let (fs, relative) = self.resolve(path)?;
        fs.remove(&relative)
    }

    pub fn rmdir(&self, path: &str) -> VfsResult<()> {
        let (fs, relative) = self.resolve(path)?;
        fs.rmdir(&relative)
    }

    pub fn readdir(&self, path: &str) -> VfsResult<Vec<DirEntry>> {
        let (fs, relative) = self.resolve(path)?;
        fs.readdir(&relative)
    }

    pub fn metadata(&self, path: &str) -> VfsResult<Metadata> {
        let (fs, relative) = self.resolve(path)?;
        fs.metadata(&relative)
    }

    pub fn exists(&self, path: &str) -> bool {
        self.metadata(path).is_ok()
    }
}

fn normalize_path(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();

    for part in path.split('/') {
        match part {
            "" | "." => continue,
            ".." => {
                parts.pop();
            }
            p => parts.push(p),
        }
    }

    if parts.is_empty() {
        "/".to_string()
    } else {
        let mut result = String::new();
        for part in parts {
            result.push('/');
            result.push_str(part);
        }
        result
    }
}

pub fn resolve_path(path: &str, cwd: &str) -> String {
    if path.starts_with('/') {
        normalize_path(path)
    } else {
        let mut full = String::from(cwd);
        if !full.ends_with('/') {
            full.push('/');
        }
        full.push_str(path);
        normalize_path(&full)
    }
}

static VFS: Lazy<Mutex<Vfs>> = Lazy::new(|| Mutex::new(Vfs::new()));

pub fn mount(path: &str, fs: Box<dyn Filesystem>) -> VfsResult<()> {
    VFS.lock().mount(path, fs)
}

pub fn unmount(path: &str) -> VfsResult<()> {
    VFS.lock().unmount(path)
}

pub fn open(path: &str, flags: OpenFlags) -> VfsResult<Box<dyn FileHandle>> {
    VFS.lock().open(path, flags)
}

pub fn mkdir(path: &str) -> VfsResult<()> {
    VFS.lock().mkdir(path)
}

pub fn remove(path: &str) -> VfsResult<()> {
    VFS.lock().remove(path)
}

pub fn rmdir(path: &str) -> VfsResult<()> {
    VFS.lock().rmdir(path)
}

pub fn readdir(path: &str) -> VfsResult<Vec<DirEntry>> {
    VFS.lock().readdir(path)
}

pub fn metadata(path: &str) -> VfsResult<Metadata> {
    VFS.lock().metadata(path)
}

pub fn exists(path: &str) -> bool {
    VFS.lock().exists(path)
}
