//! The graphical display.

use font::FONTS;
use icon::ICONS;
use console::Console;
use framebuf::FrameBuffer;

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


#[derive(Default, Clone, Copy)]
pub struct GraphicsSetting {
    pub posx:  u16,
    pub posy:  u16,
    pub font:  u8,
    pub color: [u8; 4],
}

pub struct Graphics {
    fb: FrameBuffer,
    cur: GraphicsSetting,
    saved: [GraphicsSetting; 16],
}

fn pos_from_bytes(pos: &[u8]) -> (u16, u16) {
    ((((pos[0] & 1) as u16) << 8) | (pos[1] as u16),
     (pos[0] >> 1) as u16)
}

impl Graphics {
    pub fn new(mut fb: FrameBuffer) -> Self {
        fb.clear(255);
        Self { fb, cur: Default::default(), saved: Default::default() }
    }

    pub fn process_command(&mut self, console: &Console, cmd: &[u8]) {
        let data_len = cmd.len() - 2;
        match cmd[1] {
            CMD_MODE_GRAPHICS => self.fb.activate(),
            CMD_MODE_CONSOLE  => console.activate(),
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
            CMD_TEXT => {
                self.fb.text(FONTS[self.cur.font as usize], self.cur.posx,
                             self.cur.posy, &cmd[2..], &self.cur.color);
            }
            CMD_LINES => if data_len >= 4 && data_len % 2 == 0 {
                let mut pos1 = pos_from_bytes(&cmd[2..]);
                for i in 1..data_len/2 {
                    let pos2 = pos_from_bytes(&cmd[2+2*i..]);
                    self.fb.line(pos1.0, pos1.1, pos2.0, pos2.1, self.cur.color[3]);
                    pos1 = pos2;
                }
            }
            CMD_RECT => if data_len >= 4 {
                let pos1 = pos_from_bytes(&cmd[2..]);
                let pos2 = pos_from_bytes(&cmd[4..]);
                self.fb.rect(pos1.0, pos1.1, pos2.0, pos2.1, self.cur.color[3]);
            }
            CMD_ICON => if data_len >= 1 {
                if cmd[2] < ICONS.len() as u8 {
                    let (data, size) = ICONS[cmd[2] as usize];
                    self.fb.image(self.cur.posx, self.cur.posy, data, size, &self.cur.color);
                }
            }
            CMD_CLEAR => if data_len >= 1 {
                self.fb.clear(cmd[2]);
            }
            CMD_SEL_ATTRS ..= CMD_SEL_ATTRS_MAX => {
                self.cur = self.saved[(cmd[1] - CMD_SEL_ATTRS) as usize];
            }
            CMD_SAVE_ATTRS ..= CMD_SAVE_ATTRS_MAX => {
                self.saved[(cmd[1] - CMD_SAVE_ATTRS) as usize] = self.cur;
            }
            _ => {}
        }
    }
}
