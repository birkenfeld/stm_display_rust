//! Basic display abstraction and drawing routines.

use bresenham::Bresenham;
use font::{Font, TextColors};

pub struct Display {
    pub buf: &'static mut [u8],
    pub width: u16,
    pub height: u16,
}

impl Display {
    #[inline(always)]
    fn set_pixel(&mut self, x: u16, y: u16, color: u8) {
        self.buf[x as usize + (y * self.width) as usize] = color;
    }

    pub fn clear(&mut self, color: u8) {
        // TODO: use DMAs
        for x in 0..self.width {
            for y in 0..self.height {
                self.set_pixel(x, y, color);
            }
        }
    }

    pub fn text(&mut self, font: &Font, mut px: u16, py: u16, text: &[u8], colors: &TextColors) {
        for &chr in text {
            let mut off = (chr as usize % font.n) * font.charh * font.perline();
            for y in 0..font.charh {
                for x in 0..font.charw {
                    // each pixel is encoded in 2 bit
                    let idx = off + (x >> 2);         // byte index is x/4
                    let shift = (3 - (x & 3)) << 1;   // bit shift is 2*(x%4)
                    let color = (font.data[idx] >> shift) & 3;
                    self.set_pixel(px + x as u16, py + y as u16, colors[color as usize]);
                }
                off += font.perline();
            }
            px += font.charw as u16;
        }
    }

    pub fn image(&mut self, px: u16, py: u16, img: &[u8], size: (u16, u16), color: u8) {
        for x in 0..size.0 {
            for y in 0..size.1 {
                let byte = img[(x + y*size.0) as usize / 8];
                if byte & (1 << (x % 8)) != 0 {
                    self.set_pixel(px + x, py + y, color);
                }
            }
        }
    }

    pub fn line(&mut self, x1: u16, y1: u16, x2: u16, y2: u16, color: u8) {
        for (x, y) in Bresenham::new((x1 as isize, y1 as isize), (x2 as isize, y2 as isize)) {
            self.set_pixel(x as u16, y as u16, color);
        }
    }

    pub fn rect(&mut self, x1: u16, y1: u16, x2: u16, y2: u16, color: u8) {
        // TODO: use DMAs
        for x in x1..x2+1 {
            for y in y1..y2+1 {
                self.set_pixel(x, y, color);
            }
        }
    }
}
