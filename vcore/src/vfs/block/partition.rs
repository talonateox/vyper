use super::{BlockDevice, SECTOR_SIZE};
use alloc::vec::Vec;

#[derive(Debug, Clone)]
pub struct PartitionInfo {
    pub index: u8,
    pub start_lba: u32,
    pub sector_count: u32,
    pub partition_type: PartitionType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartitionType {
    Fat32,
    Linux,
    EfiSystem,
    Unknown(u8),
}

pub fn parse_partitions<D: BlockDevice>(device: &D) -> Result<Vec<PartitionInfo>, &'static str> {
    let mut sector = [0u8; SECTOR_SIZE];
    device.read_sector(0, &mut sector)?;

    if sector[510] != 0x55 || sector[511] != 0xAA {
        return Err("no MBR signature");
    }

    if sector[450] == 0xEE {
        parse_gpt(device)
    } else {
        parse_mbr(&sector)
    }
}

fn parse_mbr(mbr: &[u8; SECTOR_SIZE]) -> Result<Vec<PartitionInfo>, &'static str> {
    let mut partitions = Vec::new();

    for i in 0..4 {
        let offset = 446 + i * 16;
        let ptype = mbr[offset + 4];

        if ptype == 0 {
            continue;
        }

        let start_lba = u32::from_le_bytes([
            mbr[offset + 8],
            mbr[offset + 9],
            mbr[offset + 10],
            mbr[offset + 11],
        ]);

        let sector_count = u32::from_le_bytes([
            mbr[offset + 12],
            mbr[offset + 13],
            mbr[offset + 14],
            mbr[offset + 15],
        ]);

        let partition_type = match ptype {
            0x0B | 0x0C => PartitionType::Fat32,
            0x83 => PartitionType::Linux,
            0xEF => PartitionType::EfiSystem,
            0xEE => continue,
            other => PartitionType::Unknown(other),
        };

        partitions.push(PartitionInfo {
            index: i as u8,
            start_lba,
            sector_count,
            partition_type,
        });
    }

    Ok(partitions)
}

fn parse_gpt<D: BlockDevice>(device: &D) -> Result<Vec<PartitionInfo>, &'static str> {
    let mut header = [0u8; SECTOR_SIZE];
    device.read_sector(1, &mut header)?;

    if &header[0..8] != b"EFI PART" {
        return Err("invalid GPT signature");
    }

    let partition_entry_lba = u64::from_le_bytes([
        header[72], header[73], header[74], header[75], header[76], header[77], header[78],
        header[79],
    ]) as u32;

    let num_entries = u32::from_le_bytes([header[80], header[81], header[82], header[83]]);

    let entry_size = u32::from_le_bytes([header[84], header[85], header[86], header[87]]);

    let mut partitions = Vec::new();
    let entries_per_sector = SECTOR_SIZE as u32 / entry_size;

    for i in 0..num_entries.min(128) {
        let sector_offset = i / entries_per_sector;
        let entry_offset = ((i % entries_per_sector) * entry_size) as usize;

        let mut sector = [0u8; SECTOR_SIZE];
        device.read_sector(partition_entry_lba + sector_offset, &mut sector)?;

        let entry = &sector[entry_offset..entry_offset + entry_size as usize];

        let type_guid = &entry[0..16];
        if type_guid.iter().all(|&b| b == 0) {
            continue;
        }

        let start_lba = u64::from_le_bytes([
            entry[32], entry[33], entry[34], entry[35], entry[36], entry[37], entry[38], entry[39],
        ]) as u32;

        let end_lba = u64::from_le_bytes([
            entry[40], entry[41], entry[42], entry[43], entry[44], entry[45], entry[46], entry[47],
        ]) as u32;

        let sector_count = end_lba - start_lba + 1;
        let partition_type = identify_gpt_type(type_guid);

        partitions.push(PartitionInfo {
            index: i as u8,
            start_lba,
            sector_count,
            partition_type,
        });
    }

    Ok(partitions)
}

fn identify_gpt_type(guid: &[u8]) -> PartitionType {
    const EFI_SYSTEM: [u8; 16] = [
        0x28, 0x73, 0x2A, 0xC1, 0x1F, 0xF8, 0xD2, 0x11, 0xBA, 0x4B, 0x00, 0xA0, 0xC9, 0x3E, 0xC9,
        0x3B,
    ];

    const BASIC_DATA: [u8; 16] = [
        0xA2, 0xA0, 0xD0, 0xEB, 0xE5, 0xB9, 0x33, 0x44, 0x87, 0xC0, 0x68, 0xB6, 0xB7, 0x26, 0x99,
        0xC7,
    ];

    const LINUX_FS: [u8; 16] = [
        0xAF, 0x3D, 0xC6, 0x0F, 0x83, 0x84, 0x72, 0x47, 0x8E, 0x79, 0x3D, 0x69, 0xD8, 0x47, 0x7D,
        0xE4,
    ];

    if guid == EFI_SYSTEM {
        PartitionType::EfiSystem
    } else if guid == BASIC_DATA {
        PartitionType::Fat32
    } else if guid == LINUX_FS {
        PartitionType::Linux
    } else {
        PartitionType::Unknown(0)
    }
}

pub fn find_partition<D: BlockDevice>(
    device: &D,
    ptype: PartitionType,
) -> Result<Option<PartitionInfo>, &'static str> {
    let partitions = parse_partitions(device)?;
    Ok(partitions.into_iter().find(|p| p.partition_type == ptype))
}

pub fn first_partition<D: BlockDevice>(device: &D) -> Result<Option<PartitionInfo>, &'static str> {
    let partitions = parse_partitions(device)?;
    Ok(partitions.into_iter().next())
}
