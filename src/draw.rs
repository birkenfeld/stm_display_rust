//! Basic display abstraction and drawing routines.

use bresenham::Bresenham;
use font::{FONTS, Font, TextColors};
use stm;

pub struct Display {
    pub buf: &'static mut [u8],
    pub width: u16,
    pub height: u16,
    pub has_cursor: bool,
}

impl Display {
    #[inline(always)]
    fn set_pixel(&mut self, x: u16, y: u16, color: u8) {
        // TODO: transparency?
        if x < self.width && y < self.height {
            self.buf[x as usize + (y * self.width) as usize] = color;
        }
    }

    pub fn text(&mut self, font: &Font, mut px: u16, py: u16, text: &[u8], colors: &TextColors) {
        for &chr in text {
            let off = ((chr as usize % font.n) * (font.charh * font.charw) as usize + 3) / 4;
            self.image(px, py, &font.data[off..], (font.charw, font.charh), colors);
            px += font.charw as u16;
        }
    }

    pub fn image(&mut self, px: u16, py: u16, img: &[u8], size: (u16, u16), colors: &TextColors) {
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

    pub fn rect(&mut self, x1: u16, y1: u16, x2: u16, y2: u16, color: u8) {
        let nx = x2 - x1;
        if !(x1 < x2 && x2 < self.width) || !(y1 < y2 && y2 < self.height) {
            return;
        }

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

    pub fn activate(&self) {
        // Color frame buffer start address
        write!(LTDC.l1cfbar: cfbadd = self.buf.as_ptr() as u32);
        // reload on next vsync
        write!(LTDC.srcr: vbr = true);
        ::enable_cursor(self.has_cursor);
    }
}

#[derive(Default)]
pub struct Graphics {
    pub cur: GraphicsSetting,
    pub saved: [GraphicsSetting; 16],
}

#[derive(Default, Clone, Copy)]
pub struct GraphicsSetting {
    pub posx:  u16,
    pub posy:  u16,
    pub font:  u8,
    pub color: [u8; 4],
}

const CMD_MODE_GRAPHICS: u8 = 0x20;
const CMD_MODE_CONSOLE:  u8 = 0x21;

const CMD_SET_POS:       u8 = 0x30;
const CMD_SET_FONT:      u8 = 0x31;
const CMD_SET_COLOR:     u8 = 0x32;

const CMD_SAVE_ATTRS:    u8 = 0x40;
const CMD_SAVE_ATTRS_MAX:u8 = 0x4f;

const CMD_SEL_ATTRS:     u8 = 0x50;
const CMD_SEL_ATTRS_MAX: u8 = 0x5f;

const CMD_TEXT:          u8 = 0x60;
const CMD_LINES:         u8 = 0x61;
const CMD_RECT:          u8 = 0x62;
const CMD_ICON:          u8 = 0x63;
const CMD_CLEAR:         u8 = 0x64;

fn pos_from_bytes(pos: &[u8]) -> (u16, u16) {
    ((((pos[0] & 1) as u16) << 8) | (pos[1] as u16),
     (pos[0] >> 1) as u16)
}

// Publicity
const MLZLOGO: &[u8] = include_bytes!("logo_mlz.dat");
const MLZLOGO_SIZE: (u16, u16) = (240, 88);
const MLZ_COLORS: [u8; 4] = [60, 104, 188, 255];

// const ARROW_UP: &[u8] = &[
//     0b10000000, 0b00000001,
//     0b11000000, 0b00000011,
//     0b11100000, 0b00000111,
//     0b11110000, 0b00001111,
//     0b11111000, 0b00011111,
//     0b11111100, 0b00111111,
//     0b11111110, 0b01111111,
//     0b11111111, 0b11111111
// ];
// const ARROW_SIZE: (u16, u16) = (16, 8);

const ICONS: &[(&[u8], (u16, u16))] = &[
    (MLZLOGO, MLZLOGO_SIZE),
];

impl Graphics {
    pub fn process(&mut self, display: &mut Display, console: &Display, seq: &[u8]) {
        let data_len = seq.len() - 2;
        match seq[1] {
            CMD_MODE_GRAPHICS => display.activate(),
            CMD_MODE_CONSOLE  => console.activate(),
            CMD_SET_POS => if data_len >= 2 {
                let (x, y) = pos_from_bytes(&seq[2..]);
                self.cur.posx = x;
                self.cur.posy = y;
            },
            CMD_SET_FONT => if data_len >= 1 {
                if seq[2] < FONTS.len() as u8 {
                    self.cur.font = seq[2];
                }
            },
            CMD_SET_COLOR => if data_len >= 4 {
                self.cur.color.copy_from_slice(&seq[2..6]);
            }
            CMD_TEXT => {
                display.text(FONTS[self.cur.font as usize], self.cur.posx,
                             self.cur.posy, &seq[2..], &self.cur.color);
            }
            CMD_LINES => if data_len >= 4 && data_len % 2 == 0 {
                let mut pos1 = pos_from_bytes(&seq[2..]);
                for i in 1..data_len/2 {
                    let pos2 = pos_from_bytes(&seq[2+2*i..]);
                    display.line(pos1.0, pos1.1, pos2.0, pos2.1, self.cur.color[3]);
                    pos1 = pos2;
                }
            }
            CMD_RECT => if data_len >= 4 {
                let pos1 = pos_from_bytes(&seq[2..]);
                let pos2 = pos_from_bytes(&seq[4..]);
                display.rect(pos1.0, pos1.1, pos2.0, pos2.1, self.cur.color[3]);
            }
            CMD_ICON => if data_len >= 1 {
                if seq[2] < ICONS.len() as u8 {
                    let (data, size) = ICONS[seq[2] as usize];
                    display.image(self.cur.posx, self.cur.posy, data, size, &self.cur.color);
                }
            }
            CMD_CLEAR => if data_len >= 1 {
                display.clear(seq[2]);
            }
            CMD_SEL_ATTRS ..= CMD_SEL_ATTRS_MAX => {
                self.cur = self.saved[(seq[1] - CMD_SEL_ATTRS) as usize];
            }
            CMD_SAVE_ATTRS ..= CMD_SAVE_ATTRS_MAX => {
                self.saved[(seq[1] - CMD_SAVE_ATTRS) as usize] = self.cur;
            }
            _ => {}
        }
    }
}
