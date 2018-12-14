//! The command interface to a client.

use crate::icon::ICONS;
use crate::console::Console;
use crate::framebuf::{FONTS, FrameBuffer};

const CMD_MODE_GRAPHICS: u8 = 0x20;
const CMD_MODE_CONSOLE:  u8 = 0x21;

const CMD_SET_POS:       u8 = 0x30;
const CMD_SET_FONT:      u8 = 0x31;
const CMD_SET_COLOR:     u8 = 0x32;
const CMD_SET_CLIP:      u8 = 0x33;

const CMD_CLEAR:         u8 = 0x40;
const CMD_LINES:         u8 = 0x41;
const CMD_RECT:          u8 = 0x42;
const CMD_ICON:          u8 = 0x43;
const CMD_TEXT:          u8 = 0x44;
const CMD_COPYRECT:      u8 = 0x45;
const CMD_PLOT:          u8 = 0x46;

const CMD_SAVE_ATTRS:    u8 = 0xa0;
const CMD_SAVE_ATTRS_MAX:u8 = 0xbf;

const CMD_SEL_ATTRS:     u8 = 0xc0;
const CMD_SEL_ATTRS_MAX: u8 = 0xdf;

const CMD_BOOTMODE:      u8 = 0xf0;
const CMD_RESET:         u8 = 0xf1;
const CMD_SET_STARTUP:   u8 = 0xf2;
const CMD_IDENT:         u8 = 0xf3;

const RESET_MAGIC:    &[u8] = &[0xcb, 0xef, 0x20, 0x18];

#[derive(Default, Clone, Copy)]
pub struct GraphicsSetting {
    pub posx:  u16,
    pub posy:  u16,
    pub clip1: (u16, u16),
    pub clip2: (u16, u16),
    pub font:  u8,
    pub color: [u8; 4],
}

pub struct DisplayState {
    gfx: FrameBuffer,
    con: Console,
    cur: GraphicsSetting,
    saved: [GraphicsSetting; 32],
    escape: Escape,
    escape_seq: [u8; 256],
}

pub enum Action<'a> {
    None,
    Reset,
    Bootloader,
    WriteEeprom(usize, usize, &'a [u8])
}

enum Escape {
    None,
    SawOne,
    Console(usize),
    Graphics(usize, usize),
}

fn pos_from_bytes(pos: &[u8]) -> (u16, u16) {
    ((((pos[0] & 1) as u16) << 8) | (pos[1] as u16),
     (pos[0] >> 1) as u16)
}

impl DisplayState {
    pub fn new(mut gfx: FrameBuffer, con: Console) -> Self {
        gfx.clear(255);
        Self { gfx, con, cur: Default::default(), saved: Default::default(),
               escape: Escape::None, escape_seq: [0; 256] }
    }

    pub fn console(&mut self) -> &mut Console {
        &mut self.con
    }

    pub fn process_byte(&mut self, ch: u8) -> Action {
        match self.escape {
            Escape::None => {
                if ch == b'\x1b' {
                    self.escape = Escape::SawOne;
                } else {
                    self.con.process_char(ch);
                }
            }
            Escape::SawOne => {
                self.escape = if ch == b'[' {
                    Escape::Console(0)
                } else if ch == b'\x1b' {
                    Escape::Graphics(0, 0)
                } else {
                    Escape::None
                };
            }
            Escape::Console(ref mut pos) => {
                if (ch >= b'0' && ch <= b'9') || ch == b';' {
                    self.escape_seq[*pos] = ch;
                    *pos += 1;
                    if *pos == self.escape_seq.len() {
                        self.escape = Escape::None;
                    }
                } else {
                    self.con.process_escape(ch, &self.escape_seq[..*pos]);
                    self.escape = Escape::None;
                }
            }
            Escape::Graphics(ref mut pos, ref mut len) => {
                if *len == 0 {
                    if ch == 0 {
                        // length of zero is not allowed
                        self.escape = Escape::None;
                        return Action::None;
                    } else {
                        *len = ch as usize + 1;
                    }
                }
                self.escape_seq[*pos] = ch;
                *pos += 1;
                if *pos == *len {
                    let escape_len = *pos;
                    self.escape = Escape::None;
                    return self.process_command(escape_len);
                }
            }
        }
        Action::None
    }

    pub fn process_command(&mut self, len: usize) -> Action {
        let cmd = &self.escape_seq[..len];
        let data_len = cmd.len() - 2;
        match cmd[1] {
            CMD_MODE_GRAPHICS => self.gfx.activate(),
            CMD_MODE_CONSOLE  => self.con.activate(),
            CMD_SET_POS => if data_len >= 2 {
                let (x, y) = pos_from_bytes(&cmd[2..]);
                self.cur.posx = x;
                self.cur.posy = y;
            },
            CMD_SET_FONT => if data_len >= 1 {
                if cmd[2] < FONTS.len() as u8 {
                    self.cur.font = cmd[2];
                }
            },
            CMD_SET_COLOR => if data_len >= 4 {
                self.cur.color.copy_from_slice(&cmd[2..6]);
            }
            CMD_SET_CLIP => {
                if data_len >= 4 {
                    self.cur.clip1 = pos_from_bytes(&cmd[2..]);
                    self.cur.clip2 = pos_from_bytes(&cmd[4..]);
                } else {
                    self.cur.clip1 = (0, 0);
                    self.cur.clip2 = (self.gfx.width(), self.gfx.height());
                }
                self.gfx.set_clip(self.cur.clip1, self.cur.clip2);
            }
            CMD_TEXT => {
                self.gfx.text(&FONTS[self.cur.font as usize], self.cur.posx,
                              self.cur.posy, &cmd[2..], &self.cur.color);
            }
            CMD_LINES => if data_len >= 4 && data_len % 2 == 0 {
                let mut pos1 = pos_from_bytes(&cmd[2..]);
                for i in 1..data_len/2 {
                    let pos2 = pos_from_bytes(&cmd[2+2*i..]);
                    self.gfx.line(pos1.0, pos1.1, pos2.0, pos2.1, self.cur.color[3]);
                    pos1 = pos2;
                }
            }
            CMD_RECT => if data_len >= 4 {
                let pos1 = pos_from_bytes(&cmd[2..]);
                let pos2 = pos_from_bytes(&cmd[4..]);
                self.gfx.rect(pos1.0, pos1.1, pos2.0, pos2.1, self.cur.color[3]);
            }
            CMD_ICON => if data_len >= 1 {
                if cmd[2] < ICONS.len() as u8 {
                    let (data, size) = ICONS[cmd[2] as usize];
                    self.gfx.image(self.cur.posx, self.cur.posy, data, size, &self.cur.color);
                }
            }
            CMD_CLEAR => if data_len >= 1 {
                self.gfx.clear(cmd[2]);
            }
            CMD_COPYRECT => if data_len >= 6 {
                let pos1 = pos_from_bytes(&cmd[2..]);
                let pos2 = pos_from_bytes(&cmd[4..]);
                let pos3 = pos_from_bytes(&cmd[6..]);
                self.gfx.copy_rect(pos1.0, pos1.1, pos2.0, pos2.1, pos3.0, pos3.1);
            }
            CMD_PLOT => if data_len >= 3 {
                let (mut x, mut y0) = pos_from_bytes(&cmd[2..]);
                for &y1 in &cmd[4..] {
                    self.gfx.line(x, y0, x+1, y1 as u16, self.cur.color[3]);
                    x += 1;
                    y0 = y1 as u16;
                }
            }
            CMD_SEL_ATTRS ..= CMD_SEL_ATTRS_MAX => {
                self.cur = self.saved[(cmd[1] - CMD_SEL_ATTRS) as usize];
                self.gfx.set_clip(self.cur.clip1, self.cur.clip2);
            }
            CMD_SAVE_ATTRS ..= CMD_SAVE_ATTRS_MAX => {
                self.saved[(cmd[1] - CMD_SAVE_ATTRS) as usize] = self.cur;
            }
            CMD_BOOTMODE => if data_len >= 4 {
                if &cmd[2..6] == RESET_MAGIC {
                    return Action::Bootloader;
                }
            },
            CMD_RESET => if data_len >= 4 {
                if &cmd[2..6] == RESET_MAGIC {
                    return Action::Reset;
                }
            },
            CMD_SET_STARTUP => {
                return Action::WriteEeprom(0, 64, &cmd[2..]);
            }
            CMD_IDENT => {
                self.con.write_to_host(&[0x1b, 0x1b, 0x04]);
                self.con.write_to_host(&crate::IDENT);
            }
            _ => {}
        }
        Action::None
    }
}
