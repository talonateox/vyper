use alloc::{boxed::Box, format, string::ToString, vec, vec::Vec};

use super::types::*;
use crate::sched::{
    SCHEDULER,
    task::{TaskMode, TaskState},
};

pub struct TasksFs;

impl TasksFs {
    pub fn new() -> Self {
        Self
    }

    fn path_parts(path: &str) -> Vec<&str> {
        path.split('/').filter(|s| !s.is_empty()).collect()
    }
}

struct TaskFileHandle {
    content: Vec<u8>,
    position: usize,
}

impl FileHandle for TaskFileHandle {
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

impl Filesystem for TasksFs {
    fn open(&self, path: &str, _flags: OpenFlags) -> VfsResult<Box<dyn FileHandle>> {
        let parts = Self::path_parts(path);

        if parts.len() != 2 {
            return Err(VfsError::IsADirectory);
        }

        let pid: u64 = parts[0].parse().map_err(|_| VfsError::NotFound)?;
        let file = parts[1];

        let guard = SCHEDULER.lock();
        let sched = guard.as_ref().ok_or(VfsError::IoError)?;

        let task = sched
            .tasks
            .iter()
            .find(|t| t.id == pid)
            .ok_or(VfsError::NotFound)?;

        let content = match file {
            "status" => {
                let state = match task.state {
                    TaskState::Ready => "ready",
                    TaskState::Running => "running",
                    TaskState::Sleeping => "sleeping",
                    TaskState::Dead => "dead",
                };
                let mode = match task.mode {
                    TaskMode::Kernel => "kernel",
                    TaskMode::User => "user",
                };
                format!("pid: {}\nstate: {}\nmode: {}", task.id, state, mode).into_bytes()
            }
            "name" => format!("{}", task.name).into_bytes(),
            _ => return Err(VfsError::NotFound),
        };

        Ok(Box::new(TaskFileHandle {
            content,
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
        let parts = Self::path_parts(path);

        match parts.len() {
            0 => {
                let guard = SCHEDULER.lock();
                let sched = guard.as_ref().ok_or(VfsError::IoError)?;

                let entries = sched
                    .tasks
                    .iter()
                    .filter(|t| t.state != TaskState::Dead)
                    .map(|t| DirEntry {
                        name: format!("{}", t.id),
                        file_type: FileType::Directory,
                    })
                    .collect();

                Ok(entries)
            }
            1 => {
                let pid: u64 = parts[0].parse().map_err(|_| VfsError::NotFound)?;

                let guard = SCHEDULER.lock();
                let sched = guard.as_ref().ok_or(VfsError::IoError)?;

                let _task = sched
                    .tasks
                    .iter()
                    .find(|t| t.id == pid)
                    .ok_or(VfsError::NotFound)?;

                Ok(vec![
                    DirEntry {
                        name: "status".to_string(),
                        file_type: FileType::File,
                    },
                    DirEntry {
                        name: "name".to_string(),
                        file_type: FileType::File,
                    },
                ])
            }
            _ => Err(VfsError::NotADirectory),
        }
    }

    fn metadata(&self, path: &str) -> VfsResult<Metadata> {
        let parts = Self::path_parts(path);

        match parts.len() {
            0 => Ok(Metadata {
                file_type: FileType::Directory,
                size: 0,
            }),
            1 => {
                let pid: u64 = parts[0].parse().map_err(|_| VfsError::NotFound)?;

                let guard = SCHEDULER.lock();
                let sched = guard.as_ref().ok_or(VfsError::IoError)?;

                let _task = sched
                    .tasks
                    .iter()
                    .find(|t| t.id == pid)
                    .ok_or(VfsError::NotFound)?;

                Ok(Metadata {
                    file_type: FileType::Directory,
                    size: 0,
                })
            }
            2 => {
                let pid: u64 = parts[0].parse().map_err(|_| VfsError::NotFound)?;
                let file = parts[1];

                let guard = SCHEDULER.lock();
                let sched = guard.as_ref().ok_or(VfsError::IoError)?;

                let _task = sched
                    .tasks
                    .iter()
                    .find(|t| t.id == pid)
                    .ok_or(VfsError::NotFound)?;

                if file == "status" || file == "name" {
                    Ok(Metadata {
                        file_type: FileType::File,
                        size: 0,
                    })
                } else {
                    Err(VfsError::NotFound)
                }
            }
            _ => Err(VfsError::NotFound),
        }
    }
}
