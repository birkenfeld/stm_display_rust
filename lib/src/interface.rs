//! The command interface to a client.

use crate::icon::ICONS;
use crate::console::{Console, WriteToHost};
use crate::framebuf::{FONTS, FrameBuffer, FbImpl};

const ESCAPE:            u8 = 0x1b;

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

const CMD_TOUCH:         u8 = 0x50;  // only for replies
const CMD_TOUCH_MODE:    u8 = 0x51;
const CMD_TOUCH_CALIB:   u8 = 0x52;

const CMD_SAVE_ATTRS:    u8 = 0xa0;
const CMD_SAVE_ATTRS_MAX:u8 = 0xbf;

const CMD_SEL_ATTRS:     u8 = 0xc0;
const CMD_SEL_ATTRS_MAX: u8 = 0xdf;

const CMD_BOOTMODE:      u8 = 0xf0;
const CMD_RESET:         u8 = 0xf1;
const CMD_SET_STARTUP:   u8 = 0xf2;
const CMD_IDENT:         u8 = 0xf3;
const CMD_RESET_APU:     u8 = 0xf4;

const BOOT_STRING:    &[u8] = b"\x1b[0mSeaBIOS ";

#[derive(Default, Clone, Copy)]
pub struct GraphicsSetting {
    pub posx:  u16,
    pub posy:  u16,
    pub clip1: (u16, u16),
    pub clip2: (u16, u16),
    pub font:  u8,
    pub color: [u8; 4],
}

pub trait TouchHandler {
    type Event;
    fn wait(&self) -> (u16, u16);
    fn convert(&self, ev: Self::Event) -> (u16, u16);
    fn set_calib(&mut self, data: (u16, u16, u16, u16));
}

pub struct DisplayState<'buf, Tx, Th, Impl> {
    gfx: FrameBuffer<'buf, Impl>,
    con: Console<'buf, Tx, Impl>,
    touch: Th,
    cur: GraphicsSetting,
    saved: [GraphicsSetting; 32],
    escape: Escape,
    escape_seq: [u8; 256],
    // if true, graphics display is currently active
    gfx_mode: bool,
    // if true, we forward touch events to the host
    // else, touch switches between graphics and console
    fwd_touch: bool,
}

pub enum Action<'a> {
    None,
    Reset,
    Bootloader,
    ResetApu,
    WriteEeprom(usize, usize, &'a [u8])
}

enum Escape {
    None,
    SawOne,
    CSI(usize),
    Graphics(usize, usize),
    MayBeBooting(usize),
}

fn pos_from_bytes(pos: &[u8]) -> (u16, u16) {
    ((((pos[0] & 1) as u16) << 8) | (pos[1] as u16),
     (pos[0] >> 1) as u16)
}

fn pos_to_bytes(x: u16, y: u16) -> (u8, u8) {
    ((y << 1) as u8 | (x >> 8) as u8, x as u8)
}

impl<'buf, Tx: WriteToHost, Th: TouchHandler, Fb: FbImpl> DisplayState<'buf, Tx, Th, Fb> {
    pub fn new(mut gfx: FrameBuffer<'buf, Fb>, con: Console<'buf, Tx, Fb>, touch: Th) -> Self {
        gfx.clear(255);
        let default_setting = GraphicsSetting {
            clip2: (gfx.width() - 1, gfx.height() - 1), .. Default::default()
        };
        Self {
            gfx, con, cur: default_setting, saved: Default::default(),
            escape: Escape::None, escape_seq: [0; 256],
            gfx_mode: false, fwd_touch: false,
            touch,
        }
    }

    pub fn is_graphics(&self) -> bool {
        self.gfx_mode
    }

    pub fn console(&mut self) -> &mut Console<'buf, Tx, Fb> {
        &mut self.con
    }

    pub fn graphics(&mut self) -> &mut FrameBuffer<'buf, Fb> {
        &mut self.gfx
    }

    pub fn split(&mut self) -> (&mut FrameBuffer<'buf, Fb>, &mut Console<'buf, Tx, Fb>, &Th) {
        (&mut self.gfx, &mut self.con, &self.touch)
    }

    pub fn process_byte(&mut self, ch: u8) -> Action {
        match self.escape {
            Escape::None => {
                if ch == ESCAPE {
                    self.escape = Escape::SawOne;
                } else {
                    self.con.process_char(ch);
                }
            }
            Escape::SawOne => {
                self.escape = match ch {
                    ESCAPE => Escape::Graphics(0, 0),
                    b'[' => Escape::CSI(0),
                    b'M' => {  // reverse linefeed = insert one line
                        self.con.process_csi(b'L', b"1");
                        Escape::None
                    }
                    _ => Escape::None,
                };
            }
            Escape::CSI(ref mut pos) => {
                if (ch >= b'0' && ch <= b'9') || ch == b';' || ch == b'?' {
                    self.escape_seq[*pos] = ch;
                    *pos += 1;
                    if *pos == self.escape_seq.len() {
                        self.escape = Escape::None;
                    }
                } else {
                    self.con.process_csi(ch, &self.escape_seq[..*pos]);
                    if ch == b'J' {
                        // Check if we're getting SeaBIOS...
                        self.escape = Escape::MayBeBooting(0);
                    } else {
                        self.escape = Escape::None;
                    }
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
            Escape::MayBeBooting(ref mut pos) => {
                *pos += 1;
                let p = *pos;
                if ch != BOOT_STRING[p - 1] {
                    self.escape = Escape::None;
                    for i in 0..p-1 {
                        self.process_byte(BOOT_STRING[i]);
                    }
                    return self.process_byte(ch);
                } else if p == BOOT_STRING.len() {
                    // We are booting! Reset the display.
                    self.escape = Escape::None;
                    return Action::Reset;
                }
            }
        }
        Action::None
    }

    pub fn process_touch(&mut self, ev: Th::Event) -> (u16, u16) {
        let (x, y) = self.touch.convert(ev);
        if self.fwd_touch {
            let (b0, b1) = pos_to_bytes(x, y);
            self.con.write_to_host(&[ESCAPE, ESCAPE, 0x03, CMD_TOUCH, b0, b1]);
        } else {
            self.gfx_mode = !self.gfx_mode;
            if self.gfx_mode {
                self.gfx.activate();
            } else {
                self.con.activate();
            }
        }
        (x, y)
    }

    pub fn process_command(&mut self, len: usize) -> Action {
        let cmd = &self.escape_seq[..len];
        let data_len = cmd.len() - 2;
        match cmd[1] {
            CMD_MODE_GRAPHICS => {
                self.gfx_mode = true;
                self.gfx.activate();
            },
            CMD_MODE_CONSOLE  => {
                self.gfx_mode = false;
                self.con.activate();
            },
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
                    self.cur.clip2 = (self.gfx.width() - 1, self.gfx.height() - 1);
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
                if &cmd[2..6] == &crate::FW_IDENT[..4] {
                    return Action::Bootloader;
                }
            },
            CMD_RESET => if data_len >= 4 {
                if &cmd[2..6] == &crate::FW_IDENT[..4] {
                    return Action::Reset;
                }
            },
            CMD_RESET_APU => if data_len >= 4 {
                if &cmd[2..6] == &crate::FW_IDENT[..4] {
                    return Action::ResetApu;
                }
            },
            CMD_SET_STARTUP => {
                return Action::WriteEeprom(0, 64, &cmd[2..]);
            }
            CMD_TOUCH_MODE => if data_len >= 1 {
                self.fwd_touch = cmd[2] > 0;
            }
            CMD_TOUCH_CALIB => if data_len >= 4 {
                self.touch.set_calib((cmd[2] as u16, cmd[3] as u16,
                                      cmd[4] as u16, cmd[5] as u16));
            }
            CMD_IDENT => {
                self.con.write_to_host(&[0x1b, 0x1b, 0x05, 0xf3]);
                self.con.write_to_host(&crate::FW_IDENT[4..]);
            }
            _ => {}
        }
        Action::None
    }
}
