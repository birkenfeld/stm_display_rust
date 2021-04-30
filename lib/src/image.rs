//! Stock image definitions.

use crate::interface::Palette;

// Each definition contains the data, the size, and the default palette.
type ImageDef = (&'static [u8], (u16, u16), Palette);

pub const IMAGES: &[ImageDef] = &[
    // Image #0 is the customer logo.
    #[cfg(feature = "customer-mlz")]
    (include_bytes!("logo_mlz.dat"), (240, 88), [15, 250, 103, 60]),
    #[cfg(feature = "customer-psi")]
    (include_bytes!("logo_psi.dat"), (244, 88), [8, 245, 251, 15]),
];
