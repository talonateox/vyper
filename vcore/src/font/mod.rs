mod glyphs;

use crate::{
    fb::DrawTarget,
    font::glyphs::{FONT_DATA, FONT_GLYPH_HEIGHT, FONT_GLYPH_WIDTH},
};

pub static FONT: Font = Font::new(&FONT_DATA, FONT_GLYPH_WIDTH, FONT_GLYPH_HEIGHT);

pub struct Font {
    data: &'static [u8],
    pub glyph_width: usize,
    pub glyph_height: usize,
}

impl Font {
    pub const fn new(data: &'static [u8], glyph_width: usize, glyph_height: usize) -> Self {
        Self {
            data,
            glyph_width,
            glyph_height,
        }
    }

    pub fn glyph(&self, c: char) -> Option<&[u8]> {
        let idx = c as usize;
        if idx >= 128 {
            return None;
        }
        let start = idx * self.glyph_height;
        let end = start + self.glyph_height;
        if end > self.data.len() {
            return None;
        }
        Some(&self.data[start..end])
    }

    pub fn draw_char<T: DrawTarget>(
        &self,
        target: &mut T,
        c: char,
        x: usize,
        y: usize,
        fg: u32,
        bg: u32,
        scale: usize,
    ) {
        let Some(glyph) = self.glyph(c) else { return };

        for (row, &byte) in glyph.iter().enumerate() {
            for col in 0..self.glyph_width {
                let color = if byte & (1 << col) != 0 { fg } else { bg };
                for sy in 0..scale {
                    for sx in 0..scale {
                        target.draw_pixel(x + col * scale + sx, y + row * scale + sy, color);
                    }
                }
            }
        }
    }
}
