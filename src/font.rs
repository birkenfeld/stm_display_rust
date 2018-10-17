use ::set_pixel;

pub type Color = [u8; 4];

pub const GRAY:  Color = [0, 235, 240, 245];
pub const WHITE: Color = [0, 239, 247, 255];
pub const RED:   Color = [0, 52, 124, 196];
pub const GREEN: Color = [0, 28, 34, 46];
pub const ALARM: Color = [1, 1, 1, 255];

pub struct Font {
    data:  &'static [u8],
    pub charw: usize,
    pub charh: usize,
    char_map: fn(u8) -> usize,
}

pub const LARGE: Font = Font {
    data:  include_bytes!("font_large.dat"),
    charw: 20,
    charh: 30,
    char_map: large_char_map,
};

pub const NORMAL: Font = Font {
    data: include_bytes!("font_terminus.dat"),
    charw: 8,
    charh: 16,
    char_map: |x| x as usize,
};

pub const CONSOLE: Font = Font {
    data: include_bytes!("font_console.dat"),
    charw: 6,
    charh: 8,
    char_map: |x| x as usize,
};

fn large_char_map(ch: u8) -> usize {
    match ch {
        b'0' ..= b'9' => (ch - b'0') as usize,
        b'+'          => 10,
        b'-'          => 11,
        b'.'          => 12,
        b'e'          => 13,
        _             => 0,
    }
}

impl Font {
    pub const fn perline(&self) -> usize {
        (self.charw + 3) / 4
    }

    pub fn draw(&self, mut px: u16, py: u16, text: &[u8], colors: &Color) {
        for &chr in text {
            let mut off = (self.char_map)(chr) * self.charh * self.perline();
            for y in 0..self.charh {
                for x in 0..self.charw {
                    let color = (self.data[off + (x >> 2)] >> (6 - 2*(x & 0b11))) & 0b11;
                    set_pixel(px + x as u16, py + y as u16, colors[color as usize]);
                }
                off += self.perline();
            }
            px += self.charw as u16;
        }
    }
}
