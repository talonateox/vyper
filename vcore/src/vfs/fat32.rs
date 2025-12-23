use alloc::{
    boxed::Box,
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};
use spin::Mutex;

use super::block::{BlockDevice, SECTOR_SIZE};
use super::types::*;

const FAT32_EOC: u32 = 0x0FFFFFF8;
const FAT32_FREE: u32 = 0x00000000;
const FAT32_BAD: u32 = 0x0FFFFFF7;

const ATTR_READ_ONLY: u8 = 0x01;
const ATTR_HIDDEN: u8 = 0x02;
const ATTR_SYSTEM: u8 = 0x04;
const ATTR_VOLUME_ID: u8 = 0x08;
const ATTR_DIRECTORY: u8 = 0x10;
const ATTR_ARCHIVE: u8 = 0x20;
const ATTR_LFN: u8 = 0x0F;

const LFN_LAST_ENTRY: u8 = 0x40;
const LFN_SEQ_MASK: u8 = 0x1F;

const DIR_ENTRY_SIZE: usize = 32;
const DELETED_MARKER: u8 = 0xE5;

#[derive(Debug, Clone)]
struct Bpb {
    bytes_per_sector: u16,
    sectors_per_cluster: u8,
    reserved_sectors: u16,
    num_fats: u8,
    total_sectors: u32,
    sectors_per_fat: u32,
    root_cluster: u32,
    fs_info_sector: u16,
}

impl Bpb {
    fn parse(sector: &[u8; SECTOR_SIZE]) -> Option<Self> {
        if sector[510] != 0x55 || sector[511] != 0xAA {
            return None;
        }

        let bytes_per_sector = u16::from_le_bytes([sector[11], sector[12]]);
        let sectors_per_cluster = sector[13];
        let reserved_sectors = u16::from_le_bytes([sector[14], sector[15]]);
        let num_fats = sector[16];

        let total_sectors_16 = u16::from_le_bytes([sector[19], sector[20]]);
        let total_sectors_32 = u32::from_le_bytes([sector[32], sector[33], sector[34], sector[35]]);
        let total_sectors = if total_sectors_16 == 0 {
            total_sectors_32
        } else {
            total_sectors_16 as u32
        };

        let sectors_per_fat = u32::from_le_bytes([sector[36], sector[37], sector[38], sector[39]]);
        let root_cluster = u32::from_le_bytes([sector[44], sector[45], sector[46], sector[47]]);
        let fs_info_sector = u16::from_le_bytes([sector[48], sector[49]]);

        Some(Self {
            bytes_per_sector,
            sectors_per_cluster,
            reserved_sectors,
            num_fats,
            total_sectors,
            sectors_per_fat,
            root_cluster,
            fs_info_sector,
        })
    }

    fn fat_start_sector(&self) -> u32 {
        self.reserved_sectors as u32
    }

    fn data_start_sector(&self) -> u32 {
        self.reserved_sectors as u32 + (self.num_fats as u32 * self.sectors_per_fat)
    }

    fn cluster_to_sector(&self, cluster: u32) -> u32 {
        self.data_start_sector() + (cluster - 2) * self.sectors_per_cluster as u32
    }

    fn bytes_per_cluster(&self) -> usize {
        self.bytes_per_sector as usize * self.sectors_per_cluster as usize
    }

    fn total_clusters(&self) -> u32 {
        (self.total_sectors - self.data_start_sector()) / self.sectors_per_cluster as u32
    }
}

#[derive(Debug, Clone)]
struct ShortDirEntry {
    name: [u8; 11],
    attr: u8,
    nt_res: u8,
    create_time_tenth: u8,
    create_time: u16,
    create_date: u16,
    access_date: u16,
    cluster_high: u16,
    modify_time: u16,
    modify_date: u16,
    cluster_low: u16,
    size: u32,
}

impl ShortDirEntry {
    fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < DIR_ENTRY_SIZE || data[0] == 0x00 {
            return None;
        }

        let mut name = [0u8; 11];
        name.copy_from_slice(&data[0..11]);

        Some(Self {
            name,
            attr: data[11],
            nt_res: data[12],
            create_time_tenth: data[13],
            create_time: u16::from_le_bytes([data[14], data[15]]),
            create_date: u16::from_le_bytes([data[16], data[17]]),
            access_date: u16::from_le_bytes([data[18], data[19]]),
            cluster_high: u16::from_le_bytes([data[20], data[21]]),
            modify_time: u16::from_le_bytes([data[22], data[23]]),
            modify_date: u16::from_le_bytes([data[24], data[25]]),
            cluster_low: u16::from_le_bytes([data[26], data[27]]),
            size: u32::from_le_bytes([data[28], data[29], data[30], data[31]]),
        })
    }

    fn serialize(&self) -> [u8; DIR_ENTRY_SIZE] {
        let mut data = [0u8; DIR_ENTRY_SIZE];
        data[0..11].copy_from_slice(&self.name);
        data[11] = self.attr;
        data[12] = self.nt_res;
        data[13] = self.create_time_tenth;
        data[14..16].copy_from_slice(&self.create_time.to_le_bytes());
        data[16..18].copy_from_slice(&self.create_date.to_le_bytes());
        data[18..20].copy_from_slice(&self.access_date.to_le_bytes());
        data[20..22].copy_from_slice(&self.cluster_high.to_le_bytes());
        data[22..24].copy_from_slice(&self.modify_time.to_le_bytes());
        data[24..26].copy_from_slice(&self.modify_date.to_le_bytes());
        data[26..28].copy_from_slice(&self.cluster_low.to_le_bytes());
        data[28..32].copy_from_slice(&self.size.to_le_bytes());
        data
    }

    fn cluster(&self) -> u32 {
        ((self.cluster_high as u32) << 16) | (self.cluster_low as u32)
    }

    fn set_cluster(&mut self, cluster: u32) {
        self.cluster_high = (cluster >> 16) as u16;
        self.cluster_low = cluster as u16;
    }

    fn is_directory(&self) -> bool {
        self.attr & ATTR_DIRECTORY != 0
    }

    fn is_volume_label(&self) -> bool {
        self.attr & ATTR_VOLUME_ID != 0
    }

    fn is_lfn(&self) -> bool {
        self.attr == ATTR_LFN
    }

    fn is_deleted(&self) -> bool {
        self.name[0] == DELETED_MARKER
    }

    fn is_free(&self) -> bool {
        self.name[0] == 0x00 || self.name[0] == DELETED_MARKER
    }

    fn short_name(&self) -> String {
        let name_part: String = self.name[0..8]
            .iter()
            .take_while(|&&c| c != b' ')
            .map(|&c| (c as char).to_ascii_lowercase())
            .collect();

        let ext_part: String = self.name[8..11]
            .iter()
            .take_while(|&&c| c != b' ')
            .map(|&c| (c as char).to_ascii_lowercase())
            .collect();

        if ext_part.is_empty() {
            name_part
        } else {
            format!("{}.{}", name_part, ext_part)
        }
    }

    fn new(name: &str, attr: u8, cluster: u32, size: u32) -> Self {
        let short_name = make_short_name(name);
        Self {
            name: short_name,
            attr,
            nt_res: 0,
            create_time_tenth: 0,
            create_time: 0,
            create_date: 0,
            access_date: 0,
            cluster_high: (cluster >> 16) as u16,
            modify_time: 0,
            modify_date: 0,
            cluster_low: cluster as u16,
            size,
        }
    }
}

#[derive(Debug, Clone)]
struct LfnEntry {
    seq: u8,
    name1: [u16; 5],
    attr: u8,
    entry_type: u8,
    checksum: u8,
    name2: [u16; 6],
    cluster: u16,
    name3: [u16; 2],
}

impl LfnEntry {
    fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < DIR_ENTRY_SIZE || data[11] != ATTR_LFN {
            return None;
        }

        let mut name1 = [0u16; 5];
        let mut name2 = [0u16; 6];
        let mut name3 = [0u16; 2];

        for i in 0..5 {
            name1[i] = u16::from_le_bytes([data[1 + i * 2], data[2 + i * 2]]);
        }
        for i in 0..6 {
            name2[i] = u16::from_le_bytes([data[14 + i * 2], data[15 + i * 2]]);
        }
        for i in 0..2 {
            name3[i] = u16::from_le_bytes([data[28 + i * 2], data[29 + i * 2]]);
        }

        Some(Self {
            seq: data[0],
            name1,
            attr: data[11],
            entry_type: data[12],
            checksum: data[13],
            name2,
            cluster: u16::from_le_bytes([data[26], data[27]]),
            name3,
        })
    }

    fn serialize(&self) -> [u8; DIR_ENTRY_SIZE] {
        let mut data = [0u8; DIR_ENTRY_SIZE];
        data[0] = self.seq;
        for i in 0..5 {
            let bytes = self.name1[i].to_le_bytes();
            data[1 + i * 2] = bytes[0];
            data[2 + i * 2] = bytes[1];
        }
        data[11] = ATTR_LFN;
        data[12] = 0;
        data[13] = self.checksum;
        for i in 0..6 {
            let bytes = self.name2[i].to_le_bytes();
            data[14 + i * 2] = bytes[0];
            data[15 + i * 2] = bytes[1];
        }
        data[26] = 0;
        data[27] = 0;
        for i in 0..2 {
            let bytes = self.name3[i].to_le_bytes();
            data[28 + i * 2] = bytes[0];
            data[29 + i * 2] = bytes[1];
        }
        data
    }

    fn sequence_number(&self) -> u8 {
        self.seq & LFN_SEQ_MASK
    }

    fn is_last(&self) -> bool {
        self.seq & LFN_LAST_ENTRY != 0
    }

    fn chars(&self) -> Vec<char> {
        let mut chars = Vec::with_capacity(13);

        for &c in &self.name1 {
            if c == 0x0000 || c == 0xFFFF {
                break;
            }
            if let Some(ch) = char::from_u32(c as u32) {
                chars.push(ch);
            }
        }
        if chars.len() < 5 {
            return chars;
        }

        for &c in &self.name2 {
            if c == 0x0000 || c == 0xFFFF {
                break;
            }
            if let Some(ch) = char::from_u32(c as u32) {
                chars.push(ch);
            }
        }
        if chars.len() < 11 {
            return chars;
        }

        for &c in &self.name3 {
            if c == 0x0000 || c == 0xFFFF {
                break;
            }
            if let Some(ch) = char::from_u32(c as u32) {
                chars.push(ch);
            }
        }

        chars
    }

    fn new(seq: u8, checksum: u8, chars: &[char], is_last: bool) -> Self {
        let mut name1 = [0xFFFFu16; 5];
        let mut name2 = [0xFFFFu16; 6];
        let mut name3 = [0xFFFFu16; 2];

        for (i, &ch) in chars.iter().take(5).enumerate() {
            name1[i] = ch as u16;
        }
        if chars.len() >= 5 {
            if chars.len() == 5 {}
            for (i, &ch) in chars.iter().skip(5).take(6).enumerate() {
                name2[i] = ch as u16;
            }
        }
        if chars.len() >= 11 {
            for (i, &ch) in chars.iter().skip(11).take(2).enumerate() {
                name3[i] = ch as u16;
            }
        }

        let len = chars.len();
        if len < 13 {
            if len < 5 {
                name1[len] = 0x0000;
            } else if len < 11 {
                name2[len - 5] = 0x0000;
            } else {
                name3[len - 11] = 0x0000;
            }
        }

        Self {
            seq: if is_last { seq | LFN_LAST_ENTRY } else { seq },
            name1,
            attr: ATTR_LFN,
            entry_type: 0,
            checksum,
            name2,
            cluster: 0,
            name3,
        }
    }
}

fn lfn_checksum(short_name: &[u8; 11]) -> u8 {
    let mut sum: u8 = 0;
    for &b in short_name {
        sum = sum.rotate_right(1).wrapping_add(b);
    }
    sum
}

fn make_short_name(name: &str) -> [u8; 11] {
    let mut result = [b' '; 11];

    let name_upper = name.to_uppercase();
    let mut parts = name_upper.rsplitn(2, '.');

    let ext = parts.next().unwrap_or("");
    let base = parts.next().unwrap_or(&name_upper);

    let (base, ext) = if parts.next().is_none() && !name.contains('.') {
        (ext, "")
    } else {
        (base, ext)
    };

    for (i, c) in base.chars().take(8).enumerate() {
        if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
            result[i] = c as u8;
        } else {
            result[i] = b'_';
        }
    }

    for (i, c) in ext.chars().take(3).enumerate() {
        if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
            result[8 + i] = c as u8;
        } else {
            result[8 + i] = b'_';
        }
    }

    result
}

fn needs_lfn(name: &str) -> bool {
    if name.len() > 12 {
        return true;
    }

    let mut parts = name.rsplitn(2, '.');
    let ext = parts.next().unwrap_or("");
    let base = parts.next();

    match base {
        Some(b) => {
            if b.len() > 8 || ext.len() > 3 {
                return true;
            }
        }
        None => {
            if ext.len() > 8 {
                return true;
            }
        }
    }

    for c in name.chars() {
        if c.is_ascii_lowercase()
            || (!c.is_ascii_alphanumeric() && c != '.' && c != '_' && c != '-')
        {
            return true;
        }
    }

    false
}

fn create_lfn_entries(name: &str, short_name: &[u8; 11]) -> Vec<LfnEntry> {
    let checksum = lfn_checksum(short_name);
    let chars: Vec<char> = name.chars().collect();
    let num_entries = (chars.len() + 12) / 13;

    let mut entries = Vec::with_capacity(num_entries);

    for i in (0..num_entries).rev() {
        let seq = (i + 1) as u8;
        let start = i * 13;
        let end = (start + 13).min(chars.len());
        let is_last = i == num_entries - 1;

        let slice = &chars[start..end];
        entries.push(LfnEntry::new(seq, checksum, slice, is_last));
    }

    entries
}

fn assemble_lfn(lfn_entries: &[LfnEntry]) -> String {
    let mut chars = Vec::new();

    for entry in lfn_entries.iter().rev() {
        chars.extend(entry.chars());
    }

    chars.into_iter().collect()
}

#[derive(Debug, Clone)]
struct FatDirEntry {
    name: String,
    short_entry: ShortDirEntry,
    lfn_entries: Vec<LfnEntry>,
    entry_offset: usize,
    entry_count: usize,
}

impl FatDirEntry {
    fn cluster(&self) -> u32 {
        self.short_entry.cluster()
    }

    fn is_directory(&self) -> bool {
        self.short_entry.is_directory()
    }

    fn size(&self) -> u32 {
        self.short_entry.size
    }
}

struct Fat32Inner<D: BlockDevice> {
    device: D,
    bpb: Bpb,
}

impl<D: BlockDevice> Fat32Inner<D> {
    fn read_sector(&self, sector: u32, buf: &mut [u8; SECTOR_SIZE]) -> Result<(), &'static str> {
        self.device.read_sector(sector, buf)
    }

    fn write_sector(&self, sector: u32, buf: &[u8; SECTOR_SIZE]) -> Result<(), &'static str> {
        self.device.write_sector(sector, buf)
    }

    fn get_fat_entry(&self, cluster: u32) -> Result<u32, &'static str> {
        let fat_offset = cluster * 4;
        let fat_sector = self.bpb.fat_start_sector() + (fat_offset / SECTOR_SIZE as u32);
        let offset = (fat_offset % SECTOR_SIZE as u32) as usize;

        let mut sector = [0u8; SECTOR_SIZE];
        self.read_sector(fat_sector, &mut sector)?;

        let entry = u32::from_le_bytes([
            sector[offset],
            sector[offset + 1],
            sector[offset + 2],
            sector[offset + 3],
        ]) & 0x0FFFFFFF;

        Ok(entry)
    }

    fn set_fat_entry(&self, cluster: u32, value: u32) -> Result<(), &'static str> {
        let fat_offset = cluster * 4;
        let offset = (fat_offset % SECTOR_SIZE as u32) as usize;

        for fat_num in 0..self.bpb.num_fats as u32 {
            let fat_sector = self.bpb.fat_start_sector()
                + (fat_num * self.bpb.sectors_per_fat)
                + (fat_offset / SECTOR_SIZE as u32);

            let mut sector = [0u8; SECTOR_SIZE];
            self.read_sector(fat_sector, &mut sector)?;

            let existing = u32::from_le_bytes([
                sector[offset],
                sector[offset + 1],
                sector[offset + 2],
                sector[offset + 3],
            ]);
            let new_value = (existing & 0xF0000000) | (value & 0x0FFFFFFF);

            let bytes = new_value.to_le_bytes();
            sector[offset..offset + 4].copy_from_slice(&bytes);

            self.write_sector(fat_sector, &sector)?;
        }

        Ok(())
    }

    fn allocate_cluster(&self) -> Result<u32, &'static str> {
        let total = self.bpb.total_clusters() + 2;

        for cluster in 2..total {
            let entry = self.get_fat_entry(cluster)?;
            if entry == FAT32_FREE {
                self.set_fat_entry(cluster, FAT32_EOC)?;
                self.zero_cluster(cluster)?;
                return Ok(cluster);
            }
        }

        Err("no free clusters")
    }

    fn allocate_chain(&self, count: usize) -> Result<u32, &'static str> {
        if count == 0 {
            return Ok(0);
        }

        let first = self.allocate_cluster()?;
        let mut prev = first;

        for _ in 1..count {
            let next = self.allocate_cluster()?;
            self.set_fat_entry(prev, next)?;
            prev = next;
        }

        Ok(first)
    }

    fn extend_chain(&self, last_cluster: u32) -> Result<u32, &'static str> {
        let new_cluster = self.allocate_cluster()?;
        self.set_fat_entry(last_cluster, new_cluster)?;
        Ok(new_cluster)
    }

    fn free_chain(&self, start: u32) -> Result<(), &'static str> {
        let mut cluster = start;

        while cluster >= 2 && cluster < FAT32_EOC {
            let next = self.get_fat_entry(cluster)?;
            self.set_fat_entry(cluster, FAT32_FREE)?;
            cluster = next;
        }

        Ok(())
    }

    fn get_last_cluster(&self, start: u32) -> Result<u32, &'static str> {
        let mut cluster = start;

        loop {
            let next = self.get_fat_entry(cluster)?;
            if next >= FAT32_EOC {
                return Ok(cluster);
            }
            cluster = next;
        }
    }

    fn zero_cluster(&self, cluster: u32) -> Result<(), &'static str> {
        let start_sector = self.bpb.cluster_to_sector(cluster);
        let zero_sector = [0u8; SECTOR_SIZE];

        for i in 0..self.bpb.sectors_per_cluster as u32 {
            self.write_sector(start_sector + i, &zero_sector)?;
        }

        Ok(())
    }

    fn read_cluster(&self, cluster: u32, buf: &mut [u8]) -> Result<(), &'static str> {
        let start_sector = self.bpb.cluster_to_sector(cluster);
        let cluster_size = self.bpb.bytes_per_cluster();

        if buf.len() < cluster_size {
            return Err("buffer too small");
        }

        for i in 0..self.bpb.sectors_per_cluster as usize {
            let mut sector = [0u8; SECTOR_SIZE];
            self.read_sector(start_sector + i as u32, &mut sector)?;
            buf[i * SECTOR_SIZE..(i + 1) * SECTOR_SIZE].copy_from_slice(&sector);
        }

        Ok(())
    }

    fn write_cluster(&self, cluster: u32, buf: &[u8]) -> Result<(), &'static str> {
        let start_sector = self.bpb.cluster_to_sector(cluster);
        let cluster_size = self.bpb.bytes_per_cluster();

        if buf.len() < cluster_size {
            return Err("buffer too small");
        }

        for i in 0..self.bpb.sectors_per_cluster as usize {
            let mut sector = [0u8; SECTOR_SIZE];
            sector.copy_from_slice(&buf[i * SECTOR_SIZE..(i + 1) * SECTOR_SIZE]);
            self.write_sector(start_sector + i as u32, &sector)?;
        }

        Ok(())
    }

    fn read_chain(&self, start: u32) -> Result<Vec<u8>, &'static str> {
        let mut data = Vec::new();
        let mut cluster = start;
        let cluster_size = self.bpb.bytes_per_cluster();
        let mut buf = vec![0u8; cluster_size];

        loop {
            if cluster < 2 || cluster >= FAT32_EOC {
                break;
            }

            self.read_cluster(cluster, &mut buf)?;
            data.extend_from_slice(&buf);

            cluster = self.get_fat_entry(cluster)?;
        }

        Ok(data)
    }

    fn write_chain(&self, start: u32, data: &[u8]) -> Result<u32, &'static str> {
        let cluster_size = self.bpb.bytes_per_cluster();
        let clusters_needed = (data.len() + cluster_size - 1) / cluster_size;

        if clusters_needed == 0 {
            return Ok(start);
        }

        let first = if start < 2 {
            self.allocate_cluster()?
        } else {
            start
        };

        let mut cluster = first;
        let mut buf = vec![0u8; cluster_size];

        for i in 0..clusters_needed {
            let offset = i * cluster_size;
            let end = (offset + cluster_size).min(data.len());

            buf.fill(0);
            buf[..end - offset].copy_from_slice(&data[offset..end]);

            self.write_cluster(cluster, &buf)?;

            if i + 1 < clusters_needed {
                let next = self.get_fat_entry(cluster)?;
                cluster = if next >= FAT32_EOC {
                    self.extend_chain(cluster)?
                } else {
                    next
                };
            }
        }

        self.set_fat_entry(cluster, FAT32_EOC)?;

        let next = self.get_fat_entry(cluster)?;
        if next >= 2 && next < FAT32_EOC {}

        Ok(first)
    }

    fn read_directory(&self, dir_cluster: u32) -> Result<Vec<FatDirEntry>, &'static str> {
        let data = self.read_chain(dir_cluster)?;
        let mut entries = Vec::new();
        let mut lfn_parts: Vec<LfnEntry> = Vec::new();
        let mut lfn_start_offset = 0;

        let mut i = 0;
        while i + DIR_ENTRY_SIZE <= data.len() {
            let entry_data = &data[i..i + DIR_ENTRY_SIZE];

            if entry_data[0] == 0x00 {
                break;
            }

            if entry_data[0] == DELETED_MARKER {
                lfn_parts.clear();
                i += DIR_ENTRY_SIZE;
                continue;
            }

            if entry_data[11] == ATTR_LFN {
                if let Some(lfn) = LfnEntry::parse(entry_data) {
                    if lfn.is_last() {
                        lfn_parts.clear();
                        lfn_start_offset = i;
                    }
                    lfn_parts.push(lfn);
                }
                i += DIR_ENTRY_SIZE;
                continue;
            }

            if let Some(short) = ShortDirEntry::parse(entry_data) {
                if short.is_volume_label() {
                    lfn_parts.clear();
                    i += DIR_ENTRY_SIZE;
                    continue;
                }

                let name = if !lfn_parts.is_empty() {
                    let expected_checksum = lfn_checksum(&short.name);
                    let valid_lfn = lfn_parts.iter().all(|e| e.checksum == expected_checksum);

                    if valid_lfn {
                        assemble_lfn(&lfn_parts)
                    } else {
                        short.short_name()
                    }
                } else {
                    short.short_name()
                };

                let short_name = short.short_name();

                if short_name != "." && short_name != ".." {
                    let entry_offset = if !lfn_parts.is_empty() {
                        lfn_start_offset
                    } else {
                        i
                    };
                    let entry_count = lfn_parts.len() + 1;

                    entries.push(FatDirEntry {
                        name,
                        short_entry: short,
                        lfn_entries: core::mem::take(&mut lfn_parts),
                        entry_offset,
                        entry_count,
                    });
                }

                lfn_parts.clear();
            }

            i += DIR_ENTRY_SIZE;
        }

        Ok(entries)
    }

    fn find_in_directory(
        &self,
        dir_cluster: u32,
        name: &str,
    ) -> Result<Option<FatDirEntry>, &'static str> {
        let entries = self.read_directory(dir_cluster)?;
        let name_lower = name.to_lowercase();

        for entry in entries {
            if entry.name.to_lowercase() == name_lower {
                return Ok(Some(entry));
            }
        }

        Ok(None)
    }

    fn find_free_dir_slots(
        &self,
        dir_cluster: u32,
        slots_needed: usize,
    ) -> Result<(Vec<u8>, usize), &'static str> {
        let mut data = self.read_chain(dir_cluster)?;
        let mut consecutive_free = 0;
        let mut start_offset = 0;

        let mut i = 0;
        while i + DIR_ENTRY_SIZE <= data.len() {
            let is_free = data[i] == 0x00 || data[i] == DELETED_MARKER;

            if is_free {
                if consecutive_free == 0 {
                    start_offset = i;
                }
                consecutive_free += 1;

                if consecutive_free >= slots_needed {
                    return Ok((data, start_offset));
                }
            } else {
                consecutive_free = 0;
            }

            i += DIR_ENTRY_SIZE;
        }

        let current_size = data.len();
        let cluster_size = self.bpb.bytes_per_cluster();

        let _last = self.get_last_cluster(dir_cluster)?;

        data.resize(current_size + cluster_size, 0);

        if consecutive_free > 0 {
            Ok((data, start_offset))
        } else {
            Ok((data, current_size))
        }
    }

    fn add_dir_entry(
        &self,
        dir_cluster: u32,
        name: &str,
        attr: u8,
        cluster: u32,
        size: u32,
    ) -> Result<(), &'static str> {
        let short_name = make_short_name(name);
        let need_lfn = needs_lfn(name);

        let lfn_entries = if need_lfn {
            create_lfn_entries(name, &short_name)
        } else {
            Vec::new()
        };

        let slots_needed = lfn_entries.len() + 1;
        let (mut data, offset) = self.find_free_dir_slots(dir_cluster, slots_needed)?;

        for (i, lfn) in lfn_entries.iter().enumerate() {
            let entry_offset = offset + i * DIR_ENTRY_SIZE;
            data[entry_offset..entry_offset + DIR_ENTRY_SIZE].copy_from_slice(&lfn.serialize());
        }

        let short_entry = ShortDirEntry::new(name, attr, cluster, size);
        let short_offset = offset + lfn_entries.len() * DIR_ENTRY_SIZE;
        data[short_offset..short_offset + DIR_ENTRY_SIZE].copy_from_slice(&short_entry.serialize());

        self.write_chain(dir_cluster, &data)?;

        Ok(())
    }

    fn update_dir_entry(&self, dir_cluster: u32, entry: &FatDirEntry) -> Result<(), &'static str> {
        let mut data = self.read_chain(dir_cluster)?;
        let short_offset = entry.entry_offset + (entry.entry_count - 1) * DIR_ENTRY_SIZE;

        if short_offset + DIR_ENTRY_SIZE > data.len() {
            return Err("invalid entry offset");
        }

        data[short_offset..short_offset + DIR_ENTRY_SIZE]
            .copy_from_slice(&entry.short_entry.serialize());

        self.write_chain(dir_cluster, &data)?;

        Ok(())
    }

    fn remove_dir_entry(&self, dir_cluster: u32, entry: &FatDirEntry) -> Result<(), &'static str> {
        let mut data = self.read_chain(dir_cluster)?;

        for i in 0..entry.entry_count {
            let offset = entry.entry_offset + i * DIR_ENTRY_SIZE;
            if offset < data.len() {
                data[offset] = DELETED_MARKER;
            }
        }

        self.write_chain(dir_cluster, &data)?;

        Ok(())
    }

    fn resolve_path(&self, path: &str) -> Result<(u32, Option<FatDirEntry>), &'static str> {
        let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

        if parts.is_empty() {
            return Ok((self.bpb.root_cluster, None));
        }

        let mut current_cluster = self.bpb.root_cluster;

        for (i, part) in parts.iter().enumerate() {
            let entry = self
                .find_in_directory(current_cluster, part)?
                .ok_or("path not found")?;

            if i == parts.len() - 1 {
                return Ok((current_cluster, Some(entry)));
            }

            if !entry.is_directory() {
                return Err("not a directory");
            }

            current_cluster = entry.cluster();
        }

        Ok((current_cluster, None))
    }

    fn split_path(&self, path: &str) -> Result<(u32, String), &'static str> {
        let path = path.trim_matches('/');

        if path.is_empty() {
            return Err("invalid path");
        }

        if let Some(pos) = path.rfind('/') {
            let parent_path = &path[..pos];
            let name = &path[pos + 1..];

            let (_, parent_entry) = self.resolve_path(parent_path)?;
            let parent_cluster = match parent_entry {
                Some(e) if e.is_directory() => e.cluster(),
                Some(_) => return Err("parent is not a directory"),
                None => self.bpb.root_cluster,
            };

            Ok((parent_cluster, name.to_string()))
        } else {
            Ok((self.bpb.root_cluster, path.to_string()))
        }
    }
}

struct FatFileHandle<D: BlockDevice + 'static> {
    fs: *const Fat32Fs<D>,
    path: String,
    cluster: u32,
    size: u32,
    position: usize,
    data: Vec<u8>,
    dirty: bool,
    flags: OpenFlags,
}

unsafe impl<D: BlockDevice + 'static> Send for FatFileHandle<D> {}
unsafe impl<D: BlockDevice + 'static> Sync for FatFileHandle<D> {}

impl<D: BlockDevice + 'static> FileHandle for FatFileHandle<D> {
    fn read(&mut self, buf: &mut [u8]) -> VfsResult<usize> {
        let available = self.data.len().saturating_sub(self.position);
        let to_read = buf.len().min(available);
        buf[..to_read].copy_from_slice(&self.data[self.position..self.position + to_read]);
        self.position += to_read;
        Ok(to_read)
    }

    fn write(&mut self, buf: &[u8]) -> VfsResult<usize> {
        if !self.flags.is_writable() {
            return Err(VfsError::PermissionDenied);
        }

        let end_pos = self.position + buf.len();
        if end_pos > self.data.len() {
            self.data.resize(end_pos, 0);
        }

        self.data[self.position..end_pos].copy_from_slice(buf);
        self.position = end_pos;
        self.dirty = true;

        Ok(buf.len())
    }

    fn seek(&mut self, pos: SeekFrom) -> VfsResult<usize> {
        let new_pos = match pos {
            SeekFrom::Start(n) => n as isize,
            SeekFrom::Current(n) => self.position as isize + n,
            SeekFrom::End(n) => self.data.len() as isize + n,
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
            size: self.data.len(),
        })
    }
}

impl<D: BlockDevice + 'static> Drop for FatFileHandle<D> {
    fn drop(&mut self) {
        if self.dirty {
            let fs = unsafe { &*self.fs };
            let _ = fs.sync_file(&self.path, self.cluster, &self.data);
        }
    }
}

pub struct Fat32Fs<D: BlockDevice + 'static> {
    inner: Mutex<Fat32Inner<D>>,
}

impl<D: BlockDevice + 'static> Fat32Fs<D> {
    pub fn new(device: D) -> Result<Self, &'static str> {
        let mut sector = [0u8; SECTOR_SIZE];
        device.read_sector(0, &mut sector)?;

        let bpb = Bpb::parse(&sector).ok_or("invalid FAT32 boot sector")?;

        if bpb.sectors_per_fat == 0 {
            return Err("not a FAT32 filesystem");
        }

        Ok(Self {
            inner: Mutex::new(Fat32Inner { device, bpb }),
        })
    }

    fn sync_file(&self, path: &str, old_cluster: u32, data: &[u8]) -> VfsResult<()> {
        let inner = self.inner.lock();

        let (parent_cluster, name) = inner.split_path(path).map_err(|_| VfsError::InvalidPath)?;

        let entry = inner
            .find_in_directory(parent_cluster, &name)
            .map_err(|_| VfsError::IoError)?
            .ok_or(VfsError::NotFound)?;

        let new_cluster = if data.is_empty() {
            if old_cluster >= 2 {
                inner
                    .free_chain(old_cluster)
                    .map_err(|_| VfsError::IoError)?;
            }
            0
        } else {
            inner
                .write_chain(old_cluster, data)
                .map_err(|_| VfsError::IoError)?
        };

        let mut updated_entry = entry;
        updated_entry.short_entry.set_cluster(new_cluster);
        updated_entry.short_entry.size = data.len() as u32;

        inner
            .update_dir_entry(parent_cluster, &updated_entry)
            .map_err(|_| VfsError::IoError)?;

        Ok(())
    }
}

impl<D: BlockDevice + 'static> Filesystem for Fat32Fs<D> {
    fn open(&self, path: &str, flags: OpenFlags) -> VfsResult<Box<dyn FileHandle>> {
        let inner = self.inner.lock();

        let (_parent_cluster, entry_opt) =
            inner.resolve_path(path).map_err(|_| VfsError::NotFound)?;

        match entry_opt {
            Some(entry) => {
                if entry.is_directory() {
                    return Err(VfsError::IsADirectory);
                }

                let data = if entry.cluster() >= 2 {
                    let mut d = inner
                        .read_chain(entry.cluster())
                        .map_err(|_| VfsError::IoError)?;
                    d.truncate(entry.size() as usize);
                    d
                } else {
                    Vec::new()
                };

                let position = if flags.contains(OpenFlags::O_APPEND) {
                    data.len()
                } else {
                    0
                };

                let data = if flags.contains(OpenFlags::O_TRUNC) {
                    Vec::new()
                } else {
                    data
                };

                drop(inner);

                Ok(Box::new(FatFileHandle {
                    fs: self as *const _,
                    path: path.to_string(),
                    cluster: entry.cluster(),
                    size: entry.size(),
                    position,
                    data,
                    dirty: flags.contains(OpenFlags::O_TRUNC),
                    flags,
                }))
            }
            None => {
                if !flags.contains(OpenFlags::O_CREAT) {
                    return Err(VfsError::NotFound);
                }

                let (parent_cluster, name) =
                    inner.split_path(path).map_err(|_| VfsError::InvalidPath)?;

                inner
                    .add_dir_entry(parent_cluster, &name, ATTR_ARCHIVE, 0, 0)
                    .map_err(|_| VfsError::IoError)?;

                drop(inner);

                Ok(Box::new(FatFileHandle {
                    fs: self as *const _,
                    path: path.to_string(),
                    cluster: 0,
                    size: 0,
                    position: 0,
                    data: Vec::new(),
                    dirty: false,
                    flags,
                }))
            }
        }
    }

    fn mkdir(&self, path: &str) -> VfsResult<()> {
        let inner = self.inner.lock();

        let (parent_cluster, name) = inner.split_path(path).map_err(|_| VfsError::InvalidPath)?;

        if inner
            .find_in_directory(parent_cluster, &name)
            .map_err(|_| VfsError::IoError)?
            .is_some()
        {
            return Err(VfsError::AlreadyExists);
        }

        let dir_cluster = inner.allocate_cluster().map_err(|_| VfsError::IoError)?;

        let cluster_size = inner.bpb.bytes_per_cluster();
        let mut dir_data = vec![0u8; cluster_size];

        let dot_entry = ShortDirEntry {
            name: *b".          ",
            attr: ATTR_DIRECTORY,
            nt_res: 0,
            create_time_tenth: 0,
            create_time: 0,
            create_date: 0,
            access_date: 0,
            cluster_high: (dir_cluster >> 16) as u16,
            modify_time: 0,
            modify_date: 0,
            cluster_low: dir_cluster as u16,
            size: 0,
        };
        dir_data[0..32].copy_from_slice(&dot_entry.serialize());

        let dotdot_entry = ShortDirEntry {
            name: *b"..         ",
            attr: ATTR_DIRECTORY,
            nt_res: 0,
            create_time_tenth: 0,
            create_time: 0,
            create_date: 0,
            access_date: 0,
            cluster_high: (parent_cluster >> 16) as u16,
            modify_time: 0,
            modify_date: 0,
            cluster_low: parent_cluster as u16,
            size: 0,
        };
        dir_data[32..64].copy_from_slice(&dotdot_entry.serialize());

        inner
            .write_cluster(dir_cluster, &dir_data)
            .map_err(|_| VfsError::IoError)?;

        inner
            .add_dir_entry(parent_cluster, &name, ATTR_DIRECTORY, dir_cluster, 0)
            .map_err(|_| VfsError::IoError)?;

        Ok(())
    }

    fn remove(&self, path: &str) -> VfsResult<()> {
        let inner = self.inner.lock();

        let (parent_cluster, name) = inner.split_path(path).map_err(|_| VfsError::InvalidPath)?;

        let entry = inner
            .find_in_directory(parent_cluster, &name)
            .map_err(|_| VfsError::IoError)?
            .ok_or(VfsError::NotFound)?;

        if entry.is_directory() {
            return Err(VfsError::IsADirectory);
        }

        if entry.cluster() >= 2 {
            inner
                .free_chain(entry.cluster())
                .map_err(|_| VfsError::IoError)?;
        }

        inner
            .remove_dir_entry(parent_cluster, &entry)
            .map_err(|_| VfsError::IoError)?;

        Ok(())
    }

    fn rmdir(&self, path: &str) -> VfsResult<()> {
        let inner = self.inner.lock();

        let (parent_cluster, name) = inner.split_path(path).map_err(|_| VfsError::InvalidPath)?;

        let entry = inner
            .find_in_directory(parent_cluster, &name)
            .map_err(|_| VfsError::IoError)?
            .ok_or(VfsError::NotFound)?;

        if !entry.is_directory() {
            return Err(VfsError::NotADirectory);
        }

        let entries = inner
            .read_directory(entry.cluster())
            .map_err(|_| VfsError::IoError)?;

        if !entries.is_empty() {
            return Err(VfsError::NotEmpty);
        }

        inner
            .free_chain(entry.cluster())
            .map_err(|_| VfsError::IoError)?;

        inner
            .remove_dir_entry(parent_cluster, &entry)
            .map_err(|_| VfsError::IoError)?;

        Ok(())
    }

    fn readdir(&self, path: &str) -> VfsResult<Vec<DirEntry>> {
        let inner = self.inner.lock();

        let cluster = if path.trim_matches('/').is_empty() {
            inner.bpb.root_cluster
        } else {
            let (_, entry) = inner.resolve_path(path).map_err(|_| VfsError::NotFound)?;

            let entry = entry.ok_or(VfsError::NotFound)?;

            if !entry.is_directory() {
                return Err(VfsError::NotADirectory);
            }

            entry.cluster()
        };

        let entries = inner
            .read_directory(cluster)
            .map_err(|_| VfsError::IoError)?;

        Ok(entries
            .into_iter()
            .map(|e| DirEntry {
                name: e.name.clone(),
                file_type: if e.is_directory() {
                    FileType::Directory
                } else {
                    FileType::File
                },
            })
            .collect())
    }

    fn metadata(&self, path: &str) -> VfsResult<Metadata> {
        let inner = self.inner.lock();

        if path.trim_matches('/').is_empty() {
            return Ok(Metadata {
                file_type: FileType::Directory,
                size: 0,
            });
        }

        let (_, entry) = inner.resolve_path(path).map_err(|_| VfsError::NotFound)?;

        let entry = entry.ok_or(VfsError::NotFound)?;

        Ok(Metadata {
            file_type: if entry.is_directory() {
                FileType::Directory
            } else {
                FileType::File
            },
            size: entry.size() as usize,
        })
    }
}
