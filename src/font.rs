//! Font handling.

pub struct Font {
    pub data:  &'static [u8],
    pub charw: u16,
    pub charh: u16,
    pub n:     usize,
}

pub const LARGE: Font = Font {
    data:  include_bytes!("font_large.dat"),
    charw: 20,
    charh: 40,
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

pub const FONTS: &[&Font] = &[&CONSOLE, &NORMAL, &LARGE];
