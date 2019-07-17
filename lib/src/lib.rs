#![no_std]

mod icon;
pub mod interface;
pub mod framebuf;
pub mod console;

/// Width and height of visible screen.
pub const WIDTH: u16 = 480;
pub const HEIGHT: u16 = 128;

/// Size of a character in the console output.
pub const CHARW: u16 = framebuf::CONSOLEFONT.size().0;
pub const CHARH: u16 = framebuf::CONSOLEFONT.size().1;

/// Reply to host's identify query.
///
/// Meanings:
///
/// - 0.0.1.0 initial version
/// - 0.0.1.1 after update to generic code with simulator
const IDENT: [u8; 4] = [0x00, 0x00, 0x01, 0x02];
