//! Basic framebuffer abstraction and drawing routines.

use stm;
use bresenham::Bresenham;

pub type Colors = [u8; 4];

pub struct Font {
    /// Bitmap data, in 2bpp.
    data:  &'static [u8],
    /// Starting pixel index of each glyph in the data.
    chars: [usize; 256],
    /// Height of every char.
    charh: u16,
    /// Width of every char.
    charw: u16,
}

impl Font {
    pub const fn size(&self) -> (u16, u16) {
        (self.charw, self.charh)
    }

    fn data(&self, chr: u8) -> &[u8] {
        &self.data[self.chars[chr as usize]..]
    }
}

pub const FONTS: &[Font] = &[
    include!("font_console.rs"),
    include!("font_medium.rs"),
    include!("font_large.rs"),
    include!("font_light.rs"),
];

pub const CONSOLEFONT: &'static Font = &FONTS[0];

pub struct FrameBuffer {
    buf: &'static mut [u8],
    width: u16,
    height: u16,
    clip1: (u16, u16),
    clip2: (u16, u16),
    has_cursor: bool,
}

impl FrameBuffer {
    pub fn new(buf: &'static mut [u8], width: u16, height: u16, has_cursor: bool) -> Self {
        Self { buf, width, height, has_cursor, clip1: (0, 0), clip2: (width, height) }
    }

    #[inline(always)]
    fn set_pixel(&mut self, x: u16, y: u16, color: u8) {
        if self.clip1.0 <= x && x < self.clip2.0 && self.clip1.1 <= y && y < self.clip2.1 {
            self.buf[x as usize + (y * self.width) as usize] = color;
        }
    }

    pub fn width(&self) -> u16 {
        self.width
    }

    pub fn height(&self) -> u16 {
        self.height
    }

    pub fn set_clip(&mut self, clip1: (u16, u16), clip2: (u16, u16)) {
        self.clip1.0 = clip1.0.min(self.width);
        self.clip1.1 = clip1.1.min(self.height);
        self.clip2.0 = clip2.0.min(self.width).max(self.clip1.0);
        self.clip2.1 = clip2.1.min(self.height).max(self.clip1.1);
    }

    pub fn text(&mut self, font: &Font, mut px: u16, py: u16, text: &[u8], colors: &Colors) {
        let size = font.size();
        for &chr in text {
            self.image(px, py, font.data(chr), size, colors);
            px += size.0;
            if px >= self.width {
                return;
            }
        }
    }

    pub fn image(&mut self, px: u16, py: u16, img: &[u8], size: (u16, u16), colors: &Colors) {
        let mut bits = 0x1;
        let mut off = 0;
        for y in 0..size.1 {
            for x in 0..size.0 {
                if bits == 0x1 {
                    bits = img[off] as u16 | 0x100;
                    off += 1;
                }
                self.set_pixel(px + x, py + y, colors[(bits & 0b11) as usize]);
                bits >>= 2;
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

    pub fn rect(&mut self, mut x1: u16, mut y1: u16, mut x2: u16, mut y2: u16, color: u8) {
        x1 = x1.max(self.clip1.0).min(self.clip2.0);
        x2 = x2.max(x1).min(self.clip2.0);
        y1 = y1.max(self.clip1.1).min(self.clip2.1);
        y2 = y2.max(y1).min(self.clip2.1);

        // Since DMA2D's smallest transfer unit is 16 bit, split off the unaligned
        // bytes here and draw them individually.
        let dma_x1 = (x1 + 1) & !1;
        let dma_x2 = x2 & !1;
        let dma_nx = dma_x2 - dma_x1;
        if dma_nx != 0 {
            write!(DMA2D.ocolr: green = color, blue = color);
            write!(DMA2D.opfccr: cm = 0b100); // ARGB4444, transfer 16bits at once
            let offset = y1*self.width + dma_x1;
            write!(DMA2D.omar: ma = self.buf.as_ptr().offset(offset as isize) as u32);
            write!(DMA2D.oor: lo = (self.width - dma_nx) >> 1);
            write!(DMA2D.nlr: pl = dma_nx >> 1, nl = y2 - y1);
            modif!(DMA2D.cr: mode = 0b11, start = true);
        }
        if dma_x1 != x1 {
            for y in y1..y2 {
                self.set_pixel(x1, y, color);
            }
        }
        if dma_x2 != x2 {
            for y in y1..y2 {
                self.set_pixel(x2 - 1, y, color);
            }
        }
        if dma_nx != 0 {
            wait_for!(DMA2D.cr: !start);
        }
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

    pub fn activate(&self) {
        // Color frame buffer start address
        write!(LTDC.l1cfbar: cfbadd = self.buf.as_ptr() as u32);
        // reload on next vsync
        write!(LTDC.srcr: vbr = true);
        ::enable_cursor(self.has_cursor);
    }

    pub fn clear_scroll_area(&mut self) {
        for el in &mut self.buf[(self.width*self.height) as usize..] { *el = 0; }
    }
}
