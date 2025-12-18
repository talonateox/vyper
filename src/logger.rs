use log::{Level, LevelFilter, Log, Metadata, Record};

use crate::{
    print, println,
    vga::{self, ColorCode},
};

static LOGGER: Logger = Logger;

pub struct Logger;

pub fn init(level: LevelFilter) -> Result<(), log::SetLoggerError> {
    log::set_logger(&LOGGER).map(|()| log::set_max_level(level))
}

impl Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let prev: ColorCode = vga::get_color();

            vga::set_color(ColorCode::new(vga::Color::LightBlue, prev.bg()));
            print!("[");
            let (color, ch) = match record.level() {
                Level::Info => (vga::Color::Green, '*'),
                Level::Warn => (vga::Color::Yellow, 'W'),
                Level::Error => (vga::Color::Red, 'E'),
                Level::Debug => (vga::Color::Pink, 'D'),
                Level::Trace => (vga::Color::DarkGray, 'T'),
            };

            vga::set_color(ColorCode::new(color, prev.bg()));
            print!("{}", ch);
            vga::set_color(ColorCode::new(vga::Color::LightBlue, prev.bg()));
            print!("] ");

            vga::set_color(ColorCode::new(vga::Color::White, prev.bg()));
            println!("{}", record.args())
        }
    }

    fn flush(&self) {}
}
