//! Basic display abstraction and drawing routines.

use bresenham::Bresenham;
use font::{Font, TextColors};
use stm;

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

    pub fn clear(&mut self, color: u8) {
        let x2 = self.width;
        let y2 = self.height;
        self.rect(0, 0, x2, y2, color);
    }

    pub fn rect(&mut self, x1: u16, y1: u16, x2: u16, y2: u16, color: u8) {
        let nx = x2 - x1;

        write!(DMA2D.ocolr: green = color, blue = color);
        write!(DMA2D.opfccr: cm = 0b100); // ARGB4444, transfer 16bits at once
        let offset = y1*self.width + ((x1 + 1) & !1);
        write!(DMA2D.omar: ma = self.buf.as_ptr().offset(offset as isize) as u32);
        write!(DMA2D.oor: lo = self.width/2 - nx/2);
        write!(DMA2D.nlr: pl = nx/2, nl = y2 - y1);
        modif!(DMA2D.cr: mode = 0b11, start = true);
        if nx % 2 == 1 {
            let x = if x1 % 2 == 1 { x1 } else { x2-1 };
            for y in y1..y2 {
                self.set_pixel(x, y, color);
            }
        }
        wait_for!(DMA2D.cr: !start);
    }

    pub fn scroll_up(&mut self, line_height: u16) {
        let offset = line_height * self.width;
        write!(DMA2D.fgmar: ma = self.buf.as_ptr().offset(offset as isize) as u32);
        write!(DMA2D.fgor: lo = 0);
        write!(DMA2D.omar: ma = self.buf.as_ptr() as u32);
        write!(DMA2D.oor: lo = 0);
        write!(DMA2D.nlr: pl = self.width, nl = self.height);
        modif!(DMA2D.cr: mode = 0, start = true);
        wait_for!(DMA2D.cr: !start);
    }
}
