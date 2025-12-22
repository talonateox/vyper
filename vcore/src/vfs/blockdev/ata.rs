use crate::{drivers, vfs::BlockDevice};

pub struct AtaBlockDevice;

impl AtaBlockDevice {
    pub fn new() -> Result<Self, &'static str> {
        Ok(Self)
    }
}

impl BlockDevice for AtaBlockDevice {
    fn read_block(&mut self, block: u64, buffer: &mut [u8]) -> Result<(), &'static str> {
        if buffer.len() < 512 {
            return Err("buffer too small");
        }

        let mut sector = [0u8; 512];
        drivers::ata::read_sector(block as u32, &mut sector)?;
        buffer[..512].copy_from_slice(&sector);
        Ok(())
    }

    fn write_block(&mut self, block: u64, buffer: &[u8]) -> Result<(), &'static str> {
        if buffer.len() < 512 {
            return Err("buffer too small");
        }

        let mut sector = [0u8; 512];
        sector.copy_from_slice(&buffer[..512]);
        drivers::ata::write_sector(block as u32, &sector)?;
        Ok(())
    }

    fn block_size(&self) -> usize {
        512
    }

    fn num_blocks(&self) -> u64 {
        131072
    }
}
