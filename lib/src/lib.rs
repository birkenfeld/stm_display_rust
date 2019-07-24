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
/// First byte is the customization for different customers.
/// Second byte is nonzero for special modes (e.g. test mode).
/// Last two bytes are the major.minor version.

/// Customers:
///
/// - 0: generic/no customization
/// - 'M': MLZ
#[cfg(not(feature = "customer-mlz"))]
const CUSTOMER: u8 = 0;
#[cfg(feature = "customer-mlz")]
const CUSTOMER: u8 = b'M';

/// Modes:
///
/// - 0: normal
/// - 1: test mode
#[cfg(not(feature = "test-mode"))]
const MODE: u8 = 0;
#[cfg(feature = "test-mode")]
const MODE: u8 = 1;

/// Changes between versions:
///
/// - 1.0: initial version
/// - 1.1: after update to generic code with simulator
/// - 1.2: after fix to reset command
/// - 1.3: after change of line/rect coordinate args
/// - 1.4: after addition of icon font
/// - 1.5: reinterpretation of the ident string
const VERSION: [u8; 2] = [1, 5];
