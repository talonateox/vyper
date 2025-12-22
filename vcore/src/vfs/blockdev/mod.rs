pub mod ata;

use alloc::{boxed::Box, string::String, vec::Vec};

use super::types::*;

pub trait BlockDevice: Send + Sync {
    fn read_block(&mut self, block: u64, buffer: &mut [u8]) -> Result<(), &'static str>;
    fn write_block(&mut self, block: u64, buffer: &[u8]) -> Result<(), &'static str>;
    fn block_size(&self) -> usize;
    fn num_blocks(&self) -> u64;
}

pub struct DevFs {
    devices: spin::Mutex<alloc::collections::BTreeMap<String, Box<dyn BlockDevice>>>,
}

impl DevFs {
    pub fn new() -> Self {
        Self {
            devices: spin::Mutex::new(alloc::collections::BTreeMap::new()),
        }
    }

    pub fn register_device(&self, name: &str, device: Box<dyn BlockDevice>) {
        self.devices.lock().insert(name.into(), device);
    }
}

struct BlockDeviceHandle {
    device_name: String,
    position: u64,
    fs_ptr: *const DevFs,
}

unsafe impl Send for BlockDeviceHandle {}
unsafe impl Sync for BlockDeviceHandle {}

impl FileHandle for BlockDeviceHandle {
    fn read(&mut self, buf: &mut [u8]) -> VfsResult<usize> {
        let fs = unsafe { &*self.fs_ptr };
        let mut devices = fs.devices.lock();
        let device = devices
            .get_mut(&self.device_name)
            .ok_or(VfsError::NotFound)?;

        let block_size = device.block_size() as u64;
        let block_num = self.position / block_size;
        let block_offset = (self.position % block_size) as usize;

        let mut temp_buf = alloc::vec![0u8; block_size as usize];
        device
            .read_block(block_num, &mut temp_buf)
            .map_err(|_| VfsError::IoError)?;

        let available = (block_size as usize).saturating_sub(block_offset);
        let to_read = buf.len().min(available);
        buf[..to_read].copy_from_slice(&temp_buf[block_offset..block_offset + to_read]);

        self.position += to_read as u64;
        Ok(to_read)
    }

    fn write(&mut self, buf: &[u8]) -> VfsResult<usize> {
        let fs = unsafe { &*self.fs_ptr };
        let mut devices = fs.devices.lock();
        let device = devices
            .get_mut(&self.device_name)
            .ok_or(VfsError::NotFound)?;

        let block_size = device.block_size() as u64;
        let block_num = self.position / block_size;
        let block_offset = (self.position % block_size) as usize;

        let mut temp_buf = alloc::vec![0u8; block_size as usize];

        if block_offset != 0 || buf.len() < block_size as usize {
            device
                .read_block(block_num, &mut temp_buf)
                .map_err(|_| VfsError::IoError)?;
        }

        let to_write = buf.len().min((block_size as usize) - block_offset);
        temp_buf[block_offset..block_offset + to_write].copy_from_slice(&buf[..to_write]);

        device
            .write_block(block_num, &temp_buf)
            .map_err(|_| VfsError::IoError)?;

        self.position += to_write as u64;
        Ok(to_write)
    }

    fn seek(&mut self, pos: SeekFrom) -> VfsResult<usize> {
        let new_pos = match pos {
            SeekFrom::Start(n) => n as i64,
            SeekFrom::Current(n) => self.position as i64 + n as i64,
            SeekFrom::End(n) => {
                let fs = unsafe { &*self.fs_ptr };
                let devices = fs.devices.lock();
                let device = devices.get(&self.device_name).ok_or(VfsError::NotFound)?;
                (device.num_blocks() * device.block_size() as u64) as i64 + n as i64
            }
        };

        if new_pos < 0 {
            return Err(VfsError::InvalidPath);
        }

        self.position = new_pos as u64;
        Ok(self.position as usize)
    }

    fn metadata(&self) -> VfsResult<Metadata> {
        let fs = unsafe { &*self.fs_ptr };
        let devices = fs.devices.lock();
        let device = devices.get(&self.device_name).ok_or(VfsError::NotFound)?;

        Ok(Metadata {
            file_type: FileType::Device,
            size: (device.num_blocks() * device.block_size() as u64) as usize,
        })
    }
}

impl Filesystem for DevFs {
    fn open(&self, path: &str, _flags: OpenFlags) -> VfsResult<Box<dyn FileHandle>> {
        let path = path.trim_start_matches('/');

        if path.is_empty() {
            return Err(VfsError::IsADirectory);
        }

        let devices = self.devices.lock();
        if !devices.contains_key(path) {
            return Err(VfsError::NotFound);
        }

        Ok(Box::new(BlockDeviceHandle {
            device_name: path.into(),
            position: 0,
            fs_ptr: self as *const DevFs,
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
        let path = path.trim_start_matches('/');

        if !path.is_empty() {
            return Err(VfsError::NotADirectory);
        }

        let devices = self.devices.lock();
        let entries = devices
            .keys()
            .map(|name| DirEntry {
                name: name.clone(),
                file_type: FileType::Device,
            })
            .collect();

        Ok(entries)
    }

    fn metadata(&self, path: &str) -> VfsResult<Metadata> {
        let path = path.trim_start_matches('/');

        if path.is_empty() {
            return Ok(Metadata {
                file_type: FileType::Directory,
                size: 0,
            });
        }

        let devices = self.devices.lock();
        let device = devices.get(path).ok_or(VfsError::NotFound)?;

        Ok(Metadata {
            file_type: FileType::Device,
            size: (device.num_blocks() * device.block_size() as u64) as usize,
        })
    }
}
