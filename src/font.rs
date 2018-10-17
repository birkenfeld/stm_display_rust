//! Font handling.

pub type TextColors = [u8; 4];

pub const GRAY:  TextColors = [0, 235, 240, 245];
pub const WHITE: TextColors = [0, 239, 247, 255];
pub const RED:   TextColors = [0, 52, 124, 196];
pub const GREEN: TextColors = [0, 28, 34, 46];
pub const ALARM: TextColors = [1, 196, 210, 255];

pub struct Font {
    pub data:  &'static [u8],
    pub charw: usize,
    pub charh: usize,
    pub n:     usize,
}

pub const LARGE: Font = Font {
    data:  include_bytes!("font_large.dat"),
    charw: 20,
    charh: 30,
    n:     128,
};

pub const NORMAL: Font = Font {
    data:  include_bytes!("font_medium.dat"),
    charw: 8,
    charh: 16,
    n:     256,
};

pub const CONSOLE: Font = Font {
    data:  include_bytes!("font_console.dat"),
    charw: 6,
    charh: 8,
    n:     256,
};

impl Font {
    #[inline(always)]
    pub const fn perline(&self) -> usize {
        (self.charw + 3) / 4
    }
}
