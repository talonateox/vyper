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
    line_spacing: usize,
    char_width: usize,
}

impl Terminal {
    pub fn new(fb: Framebuffer, font: &'static Font) -> Self {
        let scale = 2;
        let char_width = font.glyph_width * scale - 2;
        let line_spacing = 4;
        let line_height = font.glyph_height * scale + line_spacing;
        let max_x = fb.width / (font.glyph_width * scale);
        let max_y = fb.height / line_height;

        Self {
            fb,
            x: 0,
            y: 0,
            max_x,
            max_y,
            fg: 0xffffff,
            bg: 0x000000,
            scale,
            font,
            line_spacing,
            char_width,
        }
    }

    pub fn set_fg(&mut self, color: u32) {
        self.fg = color;
    }

    pub fn set_bg(&mut self, color: u32) {
        self.bg = color;
    }

    fn line_height(&self) -> usize {
        self.font.glyph_height * self.scale + self.line_spacing
    }

    fn scroll(&mut self) {
        let line_height = self.line_height();
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
        let line_height = self.line_height();
        match c {
            '\n' => self.newline(),
            '\r' => self.x = 0,
            '\t' => {
                for _ in 0..4 {
                    self.put_char(' ');
                }
            }
            '\x08' => {
                if self.x > 0 {
                    self.x -= 1;
                    self.font.draw_char(
                        &mut self.fb,
                        ' ',
                        self.x * self.char_width,
                        self.y * line_height,
                        self.fg,
                        self.bg,
                        self.scale,
                    );
                }
            }
            c if c >= ' ' => {
                if self.x >= self.max_x {
                    self.newline();
                }
                self.font.draw_char(
                    &mut self.fb,
                    c,
                    self.x * self.char_width,
                    self.y * line_height,
                    self.fg,
                    self.bg,
                    self.scale,
                );
                self.x += 1;
            }
            _ => {}
        }
    }
    pub fn clear(&mut self) {
        let pixel_count = self.fb.width * self.fb.height;
        let ptr = self.fb.address as *mut u32;
        for i in 0..pixel_count {
            unsafe {
                ptr.add(i).write_volatile(self.bg);
            }
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
