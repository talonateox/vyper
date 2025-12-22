use alloc::{boxed::Box, format, vec::Vec};

use super::types::*;
use crate::mem::{heap, pmm};

pub struct MemFs;

impl MemFs {
    pub fn new() -> Self {
        Self
    }
}

struct MemFileHandle {
    content: Vec<u8>,
    position: usize,
}

impl FileHandle for MemFileHandle {
    fn read(&mut self, buf: &mut [u8]) -> VfsResult<usize> {
        let available = self.content.len().saturating_sub(self.position);
        let to_read = buf.len().min(available);
        buf[..to_read].copy_from_slice(&self.content[self.position..self.position + to_read]);
        self.position += to_read;
        Ok(to_read)
    }

    fn write(&mut self, _buf: &[u8]) -> VfsResult<usize> {
        Err(VfsError::PermissionDenied)
    }

    fn seek(&mut self, pos: SeekFrom) -> VfsResult<usize> {
        let new_pos = match pos {
            SeekFrom::Start(n) => n as isize,
            SeekFrom::Current(n) => self.position as isize + n,
            SeekFrom::End(n) => self.content.len() as isize + n,
        };
        if new_pos < 0 {
            return Err(VfsError::InvalidPath);
        }
        self.position = new_pos as usize;
        Ok(self.position)
    }

    fn metadata(&self) -> VfsResult<Metadata> {
        Ok(Metadata {
            file_type: FileType::File,
            size: self.content.len(),
        })
    }
}

impl Filesystem for MemFs {
    fn open(&self, path: &str, _flags: OpenFlags) -> VfsResult<Box<dyn FileHandle>> {
        let path = path.trim_matches('/');
        if !path.is_empty() {
            return Err(VfsError::NotFound);
        }

        let free_pages = pmm::free_pages();
        let free_mb = (free_pages * 4096) / 1024 / 1024;
        let total_pages = pmm::total_pages();
        let total_mb = (total_pages * 4096) / 1024 / 1024;
        let used_mb = total_mb - free_mb;
        let heap_kb = heap::size();

        let content = format!(
            "total:  {} MB\nfree:   {} MB\nused:   {} MB\nheap:   {} KB\npages:  {} free / {} total\n",
            total_mb, free_mb, used_mb, heap_kb, free_pages, total_pages
        );

        Ok(Box::new(MemFileHandle {
            content: content.into_bytes(),
            position: 0,
        }))
    }

    fn mkdir(&self, _path: &str) -> VfsResult<()> {
        Err(VfsError::PermissionDenied)
    }

    fn remove(&self, _path: &str) -> VfsResult<()> {
        Err(VfsError::PermissionDenied)
    }

    fn rmdir(&self, _path: &str) -> VfsResult<()> {
        Err(VfsError::PermissionDenied)
    }

    fn readdir(&self, path: &str) -> VfsResult<Vec<DirEntry>> {
        let path = path.trim_matches('/');
        if path.is_empty() {
            Err(VfsError::NotADirectory)
        } else {
            Err(VfsError::NotFound)
        }
    }

    fn metadata(&self, path: &str) -> VfsResult<Metadata> {
        let path = path.trim_matches('/');
        if path.is_empty() {
            Ok(Metadata {
                file_type: FileType::File,
                size: 0,
            })
        } else {
            Err(VfsError::NotFound)
        }
    }
}
