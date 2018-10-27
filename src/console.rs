//! The console display

use stm;
use hal::serial::Tx;
// use hal_base::prelude::*;
use btoi::btoi;

use framebuf::{CONSOLEFONT, Colors, FrameBuffer};
use ::{WIDTH, HEIGHT, CHARW, CHARH, H_WIN_START, V_WIN_START};

const DEFAULT_COLOR: u8 = 7;
const DEFAULT_BKGRD: u8 = 0;

/// Number of characters in the visible screen.
const COLS: u16 = WIDTH / CHARW;
const ROWS: u16 = HEIGHT / CHARH;

const HEX: &[u8] = b"0123456789ABCDEF";

pub struct Console {
    fb: FrameBuffer,
    #[allow(unused)]
    tx: Tx<stm::USART1>,
    color: Colors,
    cx: u16,
    cy: u16,
}

impl Console {
    pub fn new(mut fb: FrameBuffer, tx: Tx<stm::USART1>) -> Self {
        fb.clear(0);
        fb.clear_scroll_area();
        Self { fb, tx, color: [DEFAULT_BKGRD, 0, 0, DEFAULT_COLOR], cx: 0, cy: 0 }
    }

    pub fn activate(&self) {
        self.fb.activate();
    }

    fn position_cursor(&self) {
        write!(LTDC.l2whpcr: whstpos = H_WIN_START + self.cx*CHARW + 1,
               whsppos = H_WIN_START + (self.cx + 1)*CHARW);
        write!(LTDC.l2wvpcr: wvstpos = V_WIN_START + (self.cy + 1)*CHARH,
               wvsppos = V_WIN_START + (self.cy + 1)*CHARH);
        // reload on next vsync
        write!(LTDC.srcr: vbr = true);
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
            b'\r' => {
                self.cx = 0;
            },
            b'\n' => {
                self.cx = 0;
                self.cy += 1;
                if self.cy == ROWS {
                    self.fb.scroll_up(CHARH);
                    self.cy -= 1;
                }
            },
            b'\x08' => if self.cx > 0 {
                self.cx -= 1;
                self.fb.text(CONSOLEFONT, self.cx * CHARW, self.cy * CHARH,
                             b" ", &self.color);
            },
            _ => {
                self.fb.text(CONSOLEFONT, self.cx * CHARW, self.cy * CHARH,
                             &[ch], &self.color);
                self.cx += 1;
                if self.cx >= COLS {
                    self.process_char(b'\n');
                }
            }
        }
        self.position_cursor();
        // Echo back to sender
        // block!(self.tx.write(ch)).unwrap();
    }

    pub fn process_escape(&mut self, end: u8, seq: &[u8]) {
        let mut args = seq.split(|&v| v == b';').map(|n| btoi(n).unwrap_or(0));
        match end {
            b'm' => while let Some(arg) = args.next() {
                match arg {
                    0  => { self.color[3] = DEFAULT_COLOR; self.color[0] = DEFAULT_BKGRD; }
                    // XXX should not get reset by color selection
                    1  => { self.color[3] |= 0b1000; } // XXX: only for 16colors
                    22 => { self.color[3] &= !0b1000; }
                    30...37 => { self.color[3] = arg as u8 - 30; }
                    40...47 => { self.color[0] = arg as u8 - 40; }
                    38 => { self.color[3] = args.nth(1).unwrap_or(0) as u8; }
                    48 => { self.color[0] = args.nth(1).unwrap_or(0) as u8; }
                    _ => {}
                }
            },
            b'H' | b'f' => {
                let y = args.next().unwrap_or(1);
                let x = args.next().unwrap_or(1);
                self.cx = if x > 0 { x-1 } else { 0 };
                self.cy = if y > 0 { y-1 } else { 0 };
            },
            b'A' => {
                let n = args.next().unwrap_or(1).max(1);
                self.cy -= n.min(self.cy);
            },
            b'B' => {
                let n = args.next().unwrap_or(1).max(1);
                self.cy += n.min(ROWS - self.cy - 1);
            },
            b'C' => {
                let n = args.next().unwrap_or(1).max(1);
                self.cx += n.min(COLS - self.cx - 1);
            },
            b'D' => {
                let n = args.next().unwrap_or(1).max(1);
                self.cx -= n.min(self.cx);
            },
            b'G' => {
                let x = args.next().unwrap_or(1).max(1);
                self.cx = x-1;
            }
            b'J' => {
                // TODO: process arguments
                self.fb.clear(0);
                self.cx = 0;
                self.cy = 0;
            },
            b'K' => {}, // TODO: erase line
            // otherwise, ignore
            _    => {}
        }
        self.position_cursor();
    }
}
