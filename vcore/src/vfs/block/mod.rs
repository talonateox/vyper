pub mod ata;
pub mod partition;

use alloc::{boxed::Box, collections::BTreeMap, string::String, vec::Vec};
use spin::Mutex;

use super::types::*;

pub use ata::AtaDisk;
pub use partition::{
    PartitionInfo, PartitionType, find_partition, first_partition, parse_partitions,
};

pub const SECTOR_SIZE: usize = 512;

pub trait BlockDevice: Send + Sync {
    fn read_sector(&self, lba: u32, buf: &mut [u8; SECTOR_SIZE]) -> Result<(), &'static str>;
    fn write_sector(&self, lba: u32, buf: &[u8; SECTOR_SIZE]) -> Result<(), &'static str>;

    fn sector_count(&self) -> Option<u32> {
        None
    }

    fn block_size(&self) -> usize {
        SECTOR_SIZE
    }

    fn num_blocks(&self) -> u64 {
        self.sector_count().unwrap_or(0) as u64
    }
}

pub struct Partition<D: BlockDevice> {
    device: D,
    start_lba: u32,
    sector_count: u32,
}

impl<D: BlockDevice> Partition<D> {
    pub fn new(device: D, start_lba: u32, sector_count: u32) -> Self {
        Self {
            device,
            start_lba,
            sector_count,
        }
    }

    pub fn start_lba(&self) -> u32 {
        self.start_lba
    }

    pub fn into_inner(self) -> D {
        self.device
    }
}

impl<D: BlockDevice> BlockDevice for Partition<D> {
    fn read_sector(&self, lba: u32, buf: &mut [u8; SECTOR_SIZE]) -> Result<(), &'static str> {
        if lba >= self.sector_count {
            return Err("sector out of bounds");
        }
        self.device.read_sector(self.start_lba + lba, buf)
    }

    fn write_sector(&self, lba: u32, buf: &[u8; SECTOR_SIZE]) -> Result<(), &'static str> {
        if lba >= self.sector_count {
            return Err("sector out of bounds");
        }
        self.device.write_sector(self.start_lba + lba, buf)
    }

    fn sector_count(&self) -> Option<u32> {
        Some(self.sector_count)
    }
}

pub struct DevFs {
    devices: Mutex<BTreeMap<String, Box<dyn BlockDevice>>>,
}

impl DevFs {
    pub fn new() -> Self {
        Self {
            devices: Mutex::new(BTreeMap::new()),
        }
    }

    pub fn register_device(&self, name: &str, device: Box<dyn BlockDevice>) {
        self.devices.lock().insert(name.into(), device);
    }

    pub fn unregister_device(&self, name: &str) -> Option<Box<dyn BlockDevice>> {
        self.devices.lock().remove(name)
    }
}

impl Default for DevFs {
    fn default() -> Self {
        Self::new()
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
        let devices = fs.devices.lock();
        let device = devices.get(&self.device_name).ok_or(VfsError::NotFound)?;

        let block_size = device.block_size() as u64;
        let block_num = (self.position / block_size) as u32;
        let block_offset = (self.position % block_size) as usize;

        let mut sector = [0u8; SECTOR_SIZE];
        device
            .read_sector(block_num, &mut sector)
            .map_err(|_| VfsError::IoError)?;

        let available = SECTOR_SIZE.saturating_sub(block_offset);
        let to_read = buf.len().min(available);
        buf[..to_read].copy_from_slice(&sector[block_offset..block_offset + to_read]);

        self.position += to_read as u64;
        Ok(to_read)
    }

    fn write(&mut self, buf: &[u8]) -> VfsResult<usize> {
        let fs = unsafe { &*self.fs_ptr };
        let devices = fs.devices.lock();
        let device = devices.get(&self.device_name).ok_or(VfsError::NotFound)?;

        let block_size = device.block_size() as u64;
        let block_num = (self.position / block_size) as u32;
        let block_offset = (self.position % block_size) as usize;

        let mut sector = [0u8; SECTOR_SIZE];

        if block_offset != 0 || buf.len() < SECTOR_SIZE {
            device
                .read_sector(block_num, &mut sector)
                .map_err(|_| VfsError::IoError)?;
        }

        let to_write = buf.len().min(SECTOR_SIZE - block_offset);
        sector[block_offset..block_offset + to_write].copy_from_slice(&buf[..to_write]);

        device
            .write_sector(block_num, &sector)
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
                let size = device.num_blocks() * device.block_size() as u64;
                size as i64 + n as i64
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
        let name = path.trim_start_matches('/');

        if name.is_empty() {
            return Err(VfsError::IsADirectory);
        }

        let devices = self.devices.lock();
        if !devices.contains_key(name) {
            return Err(VfsError::NotFound);
        }

        Ok(Box::new(BlockDeviceHandle {
            device_name: name.into(),
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
        Ok(devices
            .keys()
            .map(|name| DirEntry {
                name: name.clone(),
                file_type: FileType::Device,
            })
            .collect())
    }

    fn metadata(&self, path: &str) -> VfsResult<Metadata> {
        let name = path.trim_start_matches('/');

        if name.is_empty() {
            return Ok(Metadata {
                file_type: FileType::Directory,
                size: 0,
            });
        }

        let devices = self.devices.lock();
        let device = devices.get(name).ok_or(VfsError::NotFound)?;

        Ok(Metadata {
            file_type: FileType::Device,
            size: (device.num_blocks() * device.block_size() as u64) as usize,
        })
    }
}
