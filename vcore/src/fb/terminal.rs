use core::fmt::Write;

use spin::Mutex;

use crate::{fb::Framebuffer, font::Font};

pub static TERMINAL: Mutex<Option<Terminal>> = Mutex::new(None);

pub struct Terminal {
    fb: Framebuffer,
    x: usize,
    y: usize,
    max_x: usize,
    max_y: usize,
    fg: u32,
    bg: u32,
    scale: usize,
    font: &'static Font,
}

impl Terminal {
    pub fn new(fb: Framebuffer, font: &'static Font) -> Self {
        let scale = 2;
        let max_x = fb.width / (font.glyph_width * scale);
        let max_y = fb.height / (font.glyph_height * scale);

        Self {
            fb,
            x: 0,
            y: 1,
            max_x,
            max_y,
            fg: 0xffffff,
            bg: 0x000000,
            scale,
            font,
        }
    }

    pub fn set_fg(&mut self, color: u32) {
        self.fg = color;
    }

    pub fn set_bg(&mut self, color: u32) {
        self.bg = color;
    }

    fn scroll(&mut self) {
        let line_height = self.font.glyph_height * self.scale;
        let scroll_bytes = line_height * self.fb.pitch;
        let total_bytes = self.fb.height * self.fb.pitch;

        unsafe {
            core::ptr::copy(
                self.fb.address.add(scroll_bytes),
                self.fb.address,
                total_bytes - scroll_bytes,
            );

            core::ptr::write_bytes(
                self.fb.address.add(total_bytes - scroll_bytes),
                0,
                scroll_bytes,
            );
        }
    }

    fn newline(&mut self) {
        self.x = 0;
        if self.y >= self.max_y - 1 {
            self.scroll();
        } else {
            self.y += 1;
        }
    }

    pub fn put_char(&mut self, c: char) {
        match c {
            '\n' => self.newline(),
            '\r' => self.x = 0,
            '\x08' => {
                if self.x > 0 {
                    self.x -= 1;
                    self.clear();
                }
            }
            c if c >= ' ' => {
                if self.x >= self.max_x {
                    self.newline();
                }
                self.font.draw_char(
                    &mut self.fb,
                    c,
                    self.x * self.font.glyph_width * self.scale,
                    self.y * self.font.glyph_height * self.scale,
                    self.fg,
                    self.scale,
                );
                self.x += 1;
            }
            _ => {}
        }
    }
    pub fn clear(&mut self) {
        unsafe {
            core::ptr::write_bytes(self.fb.address, 0, self.fb.height * self.fb.pitch);
        }
        self.x = 0;
        self.y = 0;
    }
}

impl Write for Terminal {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for c in s.chars() {
            self.put_char(c);
        }
        Ok(())
    }
}

pub fn init(fb: Framebuffer, font: &'static Font) {
    *TERMINAL.lock() = Some(Terminal::new(fb, font));
}

pub fn set_fg(color: u32) {
    if let Some(ref mut term) = *TERMINAL.lock() {
        term.set_fg(color);
    }
}

pub fn set_bg(color: u32) {
    if let Some(ref mut term) = *TERMINAL.lock() {
        term.set_bg(color);
    }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {{
        use core::fmt::Write;
        if let Some(ref mut term) = *$crate::terminal::TERMINAL.lock() {
            let _ = write!(term, $($arg)*);
        }
    }};
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {{
        $crate::fb::terminal::set_fg(0x5555ff);
        $crate::print!("[");
        $crate::fb::terminal::set_fg(0x00ff00);
        $crate::print!("*");
        $crate::fb::terminal::set_fg(0x5555ff);
        $crate::print!("] ");
        $crate::fb::terminal::set_fg(0xffffff);
        $crate::println!($($arg)*);
    }};
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {{
        $crate::fb::terminal::set_fg(0x5555ff);
        $crate::print!("[");
        $crate::fb::terminal::set_fg(0xFFA500);
        $crate::print!("W");
        $crate::fb::terminal::set_fg(0x5555ff);
        $crate::print!("] ");
        $crate::fb::terminal::set_fg(0xffffff);
        $crate::println!($($arg)*);
    }};
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {{
        $crate::fb::terminal::set_fg(0x5555ff);
        $crate::print!("[");
        $crate::fb::terminal::set_fg(0xff0000);
        $crate::print!("E");
        $crate::fb::terminal::set_fg(0x5555ff);
        $crate::print!("] ");
        $crate::fb::terminal::set_fg(0xffffff);
        $crate::println!($($arg)*);
    }};
}
