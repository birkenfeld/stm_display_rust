#![no_std]

use pkg_version::*;

mod image;
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
/// - 'P': PSI
#[cfg(not(any(feature = "customer-mlz", feature = "customer-psi")))]
const CUSTOMER: u8 = 0;
#[cfg(feature = "customer-mlz")]
const CUSTOMER: u8 = b'M';
#[cfg(feature = "customer-psi")]
const CUSTOMER: u8 = b'P';

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
/// - 1.6: adding the identification to the binary
/// - 1.7: several fixes in firmware test mode
/// - 1.8: fix interference of startup instructions and uart
/// - 1.9: update of reinstall PXE host name
pub const VER_MAJOR: u8 = pkg_version_major!();
pub const VER_MINOR: u8 = pkg_version_minor!();

/// Identify the firmware: magic number, followed by the reply to
/// the IDENT command (4 bytes with customer, mode, version).
/// This is placed at the very end of the firmware binary.
#[link_section = ".fw_ident"]
#[export_name = "FW_IDENT"]
pub static FW_IDENT: [u8; 8] = [0xcb, 0xef, 0x20, 0x18,
                                CUSTOMER, MODE, VER_MAJOR, VER_MINOR];
