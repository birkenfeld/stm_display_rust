//! Basic framebuffer abstraction and drawing routines.

use bresenham::Bresenham;

pub type Colors = [u8; 4];

pub const BLACK_ON_WHITE: &[u8; 4] = &[15, 7, 8, 0];
pub const RED_ON_WHITE: &[u8; 4] = &[15, 217, 203, 160];
pub const GREEN_ON_WHITE: &[u8; 4] = &[15, 156, 82, 34];


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

pub const CONSOLEFONT: &Font = &FONTS[0];
pub const MEDIUMFONT: &Font = &FONTS[1];

pub trait FbImpl {
    fn fill_rect(&mut self, buf: &mut [u8], x1: u16, y1: u16, x2: u16, y2: u16, color: u8);
    fn copy_rect(&mut self, buf: &mut [u8], x1: u16, y1: u16, x2: u16, y2: u16, nx: u16, ny: u16);
    fn activate(&self, buf: &mut [u8]);
}

pub struct FrameBuffer<'buf, Fb> {
    buf: &'buf mut [u8],
    width: u16,
    height: u16,
    clip1: (u16, u16),
    clip2: (u16, u16),
    impls: Fb,
}

impl<'buf, Fb: FbImpl> FrameBuffer<'buf, Fb> {
    pub fn new(buf: &'buf mut [u8], width: u16, height: u16, impls: Fb) -> Self {
        Self { buf, width, height, impls, clip1: (0, 0), clip2: (width, height) }
    }

    pub fn buf(&self) -> &[u8] {
        &self.buf
    }

    #[inline(always)]
    fn set_pixel(&mut self, x: u16, y: u16, color: u8) {
        if self.clip1.0 <= x && x < self.clip2.0 && self.clip1.1 <= y && y < self.clip2.1 {
            self.buf.as_mut()[x as usize + (y * self.width) as usize] = color;
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

    pub fn rect_outline(&mut self, x1: u16, y1: u16, x2: u16, y2: u16, color: u8) {
        self.line(x1, y1, x1, y2, color);
        self.line(x1, y2, x2, y2, color);
        self.line(x2, y2, x2, y1, color);
        self.line(x2, y1, x1, y1, color);
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

        self.impls.fill_rect(self.buf.as_mut(), x1, y1, x2, y2, color);
    }

    pub fn copy_rect(&mut self, mut x1: u16, mut y1: u16, x2: u16, y2: u16, mut dx: u16, mut dy: u16) {
        if x1 >= self.width || y1 >= self.height || dx >= self.clip2.0 || dy >= self.clip2.1 {
            return;
        }
        let mut nx = x2.max(x1) - x1;
        let mut ny = y2.max(y1) - y1;

        if dx < self.clip1.0 {
            nx -= self.clip1.0 - dx;
            x1 += self.clip1.0 - dx;
            dx = self.clip1.0;
        }
        if dy < self.clip1.1 {
            ny -= self.clip1.1 - dy;
            y1 += self.clip1.1 - dy;
            dy = self.clip1.1;
        }
        nx = nx.min(self.clip2.0 - dx);
        ny = ny.min(self.clip2.1 - dy);

        self.impls.copy_rect(self.buf.as_mut(), x1, y1, dx, dy, nx, ny);
    }

    pub fn scroll_up(&mut self, line_height: u16) {
        self.impls.copy_rect(self.buf.as_mut(), 0, line_height, 0, 0, self.width, self.height);
    }

    pub fn clear_scroll_area(&mut self) {
        for el in &mut self.buf.as_mut()[(self.width*self.height) as usize..] { *el = 0; }
    }

    pub fn activate(&mut self) {
        self.impls.activate(self.buf.as_mut());
    }
}