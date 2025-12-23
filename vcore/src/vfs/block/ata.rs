use super::{BlockDevice, SECTOR_SIZE};
use crate::drivers;

pub struct AtaDisk;

impl AtaDisk {
    pub fn new() -> Result<Self, &'static str> {
        let mut buf = [0u8; SECTOR_SIZE];
        drivers::ata::read_sector(0, &mut buf)?;
        Ok(Self)
    }
}

impl BlockDevice for AtaDisk {
    fn read_sector(&self, lba: u32, buf: &mut [u8; SECTOR_SIZE]) -> Result<(), &'static str> {
        drivers::ata::read_sector(lba, buf)
    }

    fn write_sector(&self, lba: u32, buf: &[u8; SECTOR_SIZE]) -> Result<(), &'static str> {
        drivers::ata::write_sector(lba, buf)
    }
}
