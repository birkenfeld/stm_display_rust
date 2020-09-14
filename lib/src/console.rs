//! The console display

use btoi::btoi;

use crate::framebuf::{CONSOLEFONT, Colors, FrameBuffer, FbImpl};
use crate::{WIDTH, HEIGHT, CHARW, CHARH};

const DEFAULT_COLOR: u8 = 7;
const DEFAULT_BKGRD: u8 = 0;

/// Number of characters in the visible screen.
const COLS: u16 = WIDTH / CHARW;
const ROWS: u16 = HEIGHT / CHARH;

const HEX: &[u8] = b"0123456789ABCDEF";

pub trait WriteToHost {
    fn write_byte(&mut self, byte: u8);
}

pub struct Console<'buf, Tx, Fb> {
    fb: FrameBuffer<'buf, Fb>,
    tx: Tx,
    color: Colors,
    cx: u16,
    cy: u16,
    need_wrap: bool,
    pos_cursor: fn(u16, u16),
}

impl<'buf, Tx: WriteToHost, Fb: FbImpl> Console<'buf, Tx, Fb> {
    pub fn new(mut fb: FrameBuffer<'buf, Fb>, tx: Tx, pos_cursor: fn(u16, u16)) -> Self {
        fb.clear(0);
        fb.clear_scroll_area(0);
        Self { fb, tx, color: [DEFAULT_BKGRD, 0, 0, DEFAULT_COLOR],
               cx: 0, cy: 0, need_wrap: false, pos_cursor }
    }

    pub fn write_to_host(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.tx.write_byte(byte);
        }
    }

    pub fn buf(&self) -> &[u8] {
        &self.fb.buf()
    }

    pub fn activate(&mut self) {
        self.fb.activate();
    }

    pub fn dump_byte(&mut self, byte: u8) {
        self.process_char(HEX[(byte >> 4) as usize]);
        self.process_char(HEX[(byte & 0xf) as usize]);
    }

    #[allow(unused)]
    pub fn dump_u32(&mut self, val: u32) {
        self.dump_byte((val >> 24) as u8);
        self.dump_byte((val >> 16) as u8);
        self.dump_byte((val >>  8) as u8);
        self.dump_byte((val >>  0) as u8);
    }

    #[allow(unused)]
    pub fn process_str(&mut self, chstr: &[u8]) {
        for &ch in chstr {
            self.process_char(ch);
        }
    }

    pub fn process_char(&mut self, ch: u8) {
        match ch {
            // Carriage return
            b'\r' => {
                self.cx = 0;
                self.need_wrap = false;
            },
            // Linefeed
            b'\n' | b'\x0b' | b'\x0c' => {
                self.cx = 0;
                self.cy += 1;
                if self.cy == ROWS {
                    self.fb.scroll_up(CHARH);
                    self.cy -= 1;
                }
                self.need_wrap = false;
            },
            // Backspace
            b'\x08' => if self.cx > 0 && !self.need_wrap {
                self.cx -= 1;
            },
            // Tab
            b'\x09' => {
                self.cx = (self.cx & !0b111) + 8;
                if self.cx >= COLS {
                    self.cx = COLS - 1;
                    self.need_wrap = true;
                }
            }
            // Ignored control characters
            b'\x00' | b'\x07' | b'\x0e' | b'\x0f' => (),
            // Any other character is echoed literally.
            _ => {
                if self.need_wrap {
                    self.process_char(b'\n');
                }
                self.fb.text(CONSOLEFONT, self.cx * CHARW, self.cy * CHARH,
                             &[ch], &self.color);
                if self.cx < COLS - 1 {
                    self.cx += 1;
                } else {
                    self.need_wrap = true;
                }
            }
        }
        (self.pos_cursor)(self.cx, self.cy);
    }

    pub fn process_csi(&mut self, end: u8, seq: &[u8]) {
        let mut args = seq.split(|&v| v == b';').map(|n| btoi(n).unwrap_or(0));
        match end {
            b'm' => while let Some(arg) = args.next() {
                match arg {
                    0  => { self.color[3] = DEFAULT_COLOR; self.color[0] = DEFAULT_BKGRD; }
                    // XXX should not get reset by color selection
                    1  => { self.color[3] |= 0b1000; } // XXX: only for 16colors
                    7  => { self.color.swap(0, 3); }
                    22 => { self.color[3] &= !0b1000; }
                    30..=37 => { self.color[3] = arg as u8 - 30; }
                    40..=47 => { self.color[0] = arg as u8 - 40; }
                    38 => { self.color[3] = args.nth(1).unwrap_or(0) as u8; }
                    48 => { self.color[0] = args.nth(1).unwrap_or(0) as u8; }
                    _ => {}
                }
            },
            b'H' | b'f' => {  // position cursor
                let y = args.next().unwrap_or(1).min(ROWS);
                let x = args.next().unwrap_or(1).min(COLS);
                self.cx = if x > 0 { x-1 } else { 0 };
                self.cy = if y > 0 { y-1 } else { 0 };
                self.need_wrap = false;
            }
            b'G' | b'`' => {  // move cursor to given column
                let x = args.next().unwrap_or(1).max(1).min(COLS);
                self.cx = x-1;
                self.need_wrap = false;
            }
            b'd' => {  // move cursor to given row
                let y = args.next().unwrap_or(1).max(1).min(ROWS);
                self.cy = y-1;
                self.need_wrap = false;
            }
            b'A' => {  // move cursor up
                let n = args.next().unwrap_or(1).max(1);
                self.cy -= n.min(self.cy);
                self.need_wrap = false;
            }
            b'B' | b'e' => {  // move cursor down
                let n = args.next().unwrap_or(1).max(1);
                self.cy += n.min(ROWS - self.cy - 1);
                self.need_wrap = false;
            }
            b'C' | b'a' => {  // move cursor right
                let n = args.next().unwrap_or(1).max(1);
                self.cx += n.min(COLS - self.cx - 1);
                self.need_wrap = false;
            }
            b'D' => {  // move cursor left
                let n = args.next().unwrap_or(1).max(1);
                self.cx -= n.min(self.cx);
                self.need_wrap = false;
            }
            b'J' => {  // erase screen
                let arg = args.next().unwrap_or(0);
                let (px, py) = (self.cx * CHARW, self.cy * CHARH);
                if arg == 0 {  // from cursor
                    self.fb.rect(px, py, WIDTH - 1, py + CHARH - 1, 0);
                    if self.cy < ROWS - 1 {
                        self.fb.rect(0, py + CHARH, WIDTH - 1, HEIGHT - 1, 0);
                    }
                } else if arg == 1 {  // to cursor
                    self.fb.rect(0, py, px + CHARW - 1, py + CHARH - 1, 0);
                    if self.cy > 0 {
                        self.fb.rect(0, 0, WIDTH - 1, py - 1, 0);
                    }
                } else {  // entire screen
                    self.fb.clear(0);
                }
            }
            b'K' => {  // erase line
                let arg = args.next().unwrap_or(0);
                let (px, py) = (self.cx * CHARW, self.cy * CHARH);
                if arg == 0 {  // from cursor
                    self.fb.rect(px, py, WIDTH - 1, py + CHARH - 1, 0);
                } else if arg == 1 {  // to cursor
                    self.fb.rect(0, py, px + CHARW - 1, py + CHARH - 1, 0);
                } else {  // entire line
                    self.fb.rect(0, py, WIDTH - 1, py + CHARH - 1, 0);
                }
            },
            b'L' => {  // insert some lines
                let n = args.next().unwrap_or(1).max(1).min(ROWS - self.cy);
                let py = self.cy * CHARH;
                if self.cy < ROWS - n {
                    for i in (0..ROWS - n - self.cy).rev() {
                        self.fb.copy_rect(0, py + i*CHARH, WIDTH - 1, py + (i + 1)*CHARH - 1,
                                          0, py + (i + n)*CHARH);
                    }
                }
                self.fb.rect(0, py, WIDTH - 1, py + n*CHARH - 1, 0);
            }
            b'M' => {  // delete some lines
                let n = args.next().unwrap_or(1).max(1).min(ROWS - self.cy);
                let py = self.cy * CHARH;
                if self.cy < ROWS - 1 {
                    self.fb.copy_rect(0, py + CHARH, WIDTH - 1, HEIGHT - 1, 0, py);
                }
                self.fb.rect(0, HEIGHT - n*CHARH, WIDTH - 1, HEIGHT - 1, 0);
            }
            b'P' => {  // delete some chars
                let n = args.next().unwrap_or(1).max(1).min(COLS - self.cx);
                let (px, py) = (self.cx * CHARW, self.cy * CHARH);
                self.fb.copy_rect(px + n*CHARW, py, WIDTH - 1, py + CHARH - 1, px, py);
                self.fb.rect(WIDTH - n*CHARW, py, WIDTH - 1, py + CHARH - 1, 0);
            }
            b'X' => {  // erase some chars
                let n = args.next().unwrap_or(1).max(1).min(COLS - self.cx);
                let (px, py) = (self.cx * CHARW, self.cy * CHARH);
                self.fb.rect(px, py, px + n*CHARW - 1, py + CHARH - 1, 0);
            }
            b'@' => {  // insert some blanks
                let _n = args.next().unwrap_or(1).max(1);
                // TODO implement this
            }
            // otherwise, ignore
            _    => {}
        }
        (self.pos_cursor)(self.cx, self.cy);
    }
}

pub fn get_lut_colors() -> impl Iterator<Item=(u8, u8, u8)> {
    let basic_16 = (0..16).map(|v| {
        let b = (v & 4 != 0) as u8;
        let g = (v & 2 != 0) as u8;
        let r = (v & 1 != 0) as u8;
        let i = (v & 8 != 0) as u8;
        (0x55*(r<<1 | i), 0x55*(g<<1 | i), 0x55*(b<<1 | i))
    });
    let colorcube = (0..6).flat_map(move |r| {
        (0..6).flat_map(move |g| {
            (0..6).map(move |b| (0x33*r, 0x33*g, 0x33*b))
        })
    });
    let grayscale = (0..24).map(|g| (8+10*g, 8+10*g, 8+10*g));

    basic_16.chain(colorcube).chain(grayscale)
}
