use spin::Mutex;
use x86_64::instructions::port::{Port, PortReadOnly, PortWriteOnly};

const ATA_PRIMARY_DATA: u16 = 0x1F0;
const ATA_PRIMARY_ERROR: u16 = 0x1F1;
const ATA_PRIMARY_SECTOR_COUNT: u16 = 0x1F2;
const ATA_PRIMARY_LBA_LOW: u16 = 0x1F3;
const ATA_PRIMARY_LBA_MID: u16 = 0x1F4;
const ATA_PRIMARY_LBA_HIGH: u16 = 0x1F5;
const ATA_PRIMARY_DRIVE_SELECT: u16 = 0x1F6;
const ATA_PRIMARY_COMMAND: u16 = 0x1F7;
const ATA_PRIMARY_STATUS: u16 = 0x1F7;

const ATA_CMD_READ_SECTORS: u8 = 0x20;
const ATA_CMD_WRITE_SECTORS: u8 = 0x30;
const ATA_CMD_IDENTIFY: u8 = 0xEC;

const ATA_SR_BSY: u8 = 0x80;
const ATA_SR_DRDY: u8 = 0x40;
const ATA_SR_DRQ: u8 = 0x08;
const ATA_SR_ERR: u8 = 0x01;

pub struct AtaDrive {
    data: Port<u16>,
    error: PortReadOnly<u8>,
    sector_count: Port<u8>,
    lba_low: Port<u8>,
    lba_mid: Port<u8>,
    lba_high: Port<u8>,
    drive_select: Port<u8>,
    command: PortWriteOnly<u8>,
    status: PortReadOnly<u8>,
}

impl AtaDrive {
    fn new() -> Self {
        Self {
            data: Port::new(ATA_PRIMARY_DATA),
            error: PortReadOnly::new(ATA_PRIMARY_ERROR),
            sector_count: Port::new(ATA_PRIMARY_SECTOR_COUNT),
            lba_low: Port::new(ATA_PRIMARY_LBA_LOW),
            lba_mid: Port::new(ATA_PRIMARY_LBA_MID),
            lba_high: Port::new(ATA_PRIMARY_LBA_HIGH),
            drive_select: Port::new(ATA_PRIMARY_DRIVE_SELECT),
            command: PortWriteOnly::new(ATA_PRIMARY_COMMAND),
            status: PortReadOnly::new(ATA_PRIMARY_STATUS),
        }
    }

    fn wait_ready(&mut self) -> Result<(), &'static str> {
        for _ in 0..1000000 {
            let status = unsafe { self.status.read() };
            if status & ATA_SR_BSY == 0 {
                return Ok(());
            }
        }
        Err("ATA timeout waiting for ready")
    }

    fn wait_data(&mut self) -> Result<(), &'static str> {
        for _ in 0..1000000 {
            let status = unsafe { self.status.read() };
            if status & ATA_SR_DRQ != 0 {
                return Ok(());
            }
            if status & ATA_SR_ERR != 0 {
                return Err("ATA error");
            }
        }
        Err("ATA timeout waiting for data")
    }

    pub fn read_sector(&mut self, lba: u32, buffer: &mut [u8; 512]) -> Result<(), &'static str> {
        self.wait_ready()?;

        unsafe {
            self.drive_select.write(0xE0 | ((lba >> 24) & 0x0F) as u8);
            self.sector_count.write(1);
            self.lba_low.write((lba & 0xFF) as u8);
            self.lba_mid.write(((lba >> 8) & 0xFF) as u8);
            self.lba_high.write(((lba >> 16) & 0xFF) as u8);
            self.command.write(ATA_CMD_READ_SECTORS);
        }

        self.wait_data()?;

        unsafe {
            for i in 0..256 {
                let word = self.data.read();
                buffer[i * 2] = (word & 0xFF) as u8;
                buffer[i * 2 + 1] = (word >> 8) as u8;
            }
        }

        Ok(())
    }

    pub fn write_sector(&mut self, lba: u32, buffer: &[u8; 512]) -> Result<(), &'static str> {
        self.wait_ready()?;

        unsafe {
            self.drive_select.write(0xE0 | ((lba >> 24) & 0x0F) as u8);
            self.sector_count.write(1);
            self.lba_low.write((lba & 0xFF) as u8);
            self.lba_mid.write(((lba >> 8) & 0xFF) as u8);
            self.lba_high.write(((lba >> 16) & 0xFF) as u8);
            self.command.write(ATA_CMD_WRITE_SECTORS);
        }

        self.wait_data()?;

        unsafe {
            for i in 0..256 {
                let word = buffer[i * 2] as u16 | ((buffer[i * 2 + 1] as u16) << 8);
                self.data.write(word);
            }
        }

        self.wait_ready()?;
        Ok(())
    }

    fn identify(&mut self) -> Result<[u16; 256], &'static str> {
        self.wait_ready()?;

        unsafe {
            self.drive_select.write(0xA0);
            self.sector_count.write(0);
            self.lba_low.write(0);
            self.lba_mid.write(0);
            self.lba_high.write(0);
            self.command.write(ATA_CMD_IDENTIFY);
        }

        let status = unsafe { self.status.read() };
        if status == 0 {
            return Err("no drive detected");
        }

        self.wait_data()?;

        let mut identify_data = [0u16; 256];
        unsafe {
            for i in 0..256 {
                identify_data[i] = self.data.read();
            }
        }

        Ok(identify_data)
    }
}

static ATA: Mutex<Option<AtaDrive>> = Mutex::new(None);

pub fn init() -> Result<(), &'static str> {
    let mut drive = AtaDrive::new();

    match drive.identify() {
        Ok(_) => {
            *ATA.lock() = Some(drive);
            Ok(())
        }
        Err(e) => Err(e),
    }
}

pub fn read_sector(lba: u32, buffer: &mut [u8; 512]) -> Result<(), &'static str> {
    ATA.lock()
        .as_mut()
        .ok_or("ATA not initialized")?
        .read_sector(lba, buffer)
}

pub fn write_sector(lba: u32, buffer: &[u8; 512]) -> Result<(), &'static str> {
    ATA.lock()
        .as_mut()
        .ok_or("ATA not initialized")?
        .write_sector(lba, buffer)
}
