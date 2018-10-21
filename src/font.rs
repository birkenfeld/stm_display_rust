//! Font handling.

pub struct Font {
    /// Bitmap data, in 2bpp.
    data:  &'static [u8],
    /// Starting pixel index of each glyph in the data.
    chars: &'static [usize; 256],
    /// Height of every char.
    charh: u16,
    /// Width of every char.
    charw: u16,
}

impl Font {
    pub const fn size(&self) -> (u16, u16) {
        (self.charw, self.charh)
    }

    pub fn data(&self, chr: u8) -> &[u8] {
        &self.data[self.chars[chr as usize]..]
    }
}

pub const LARGE: Font = Font {
    data:  include_bytes!("font_large.dat"),
    chars: &include!("font_large.idx"),
    charw: 20,
    charh: 40,
};

pub const NORMAL: Font = Font {
    data:  include_bytes!("font_medium.dat"),
    chars: &include!("font_medium.idx"),
    charw: 8,
    charh: 16,
};

pub const CONSOLE: Font = Font {
    data:  include_bytes!("font_console.dat"),
    chars: &include!("font_console.idx"),
    charw: 6,
    charh: 8,
};

pub const FONTS: &[&Font] = &[&CONSOLE, &NORMAL, &LARGE];
