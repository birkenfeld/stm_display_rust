//! Icon definitions.

// data, size, default palette
type IconDef = (&'static [u8], (u16, u16), [u8; 4]);

pub const ICONS: &[IconDef] = &[
    #[cfg(feature = "customer-mlz")]
    (include_bytes!("logo_mlz.dat"), (240, 88), [15, 250, 103, 60]),
    #[cfg(feature = "customer-psi")]
    (include_bytes!("logo_psi.dat"), (244, 88), [8, 245, 251, 15]),
];
