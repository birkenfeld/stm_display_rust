//! Icon definitions.

type IconDef = (&'static [u8], (u16, u16));

pub const ICONS: &[IconDef] = &[
    #[cfg(feature = "customer-mlz")]
    (include_bytes!("logo_mlz.dat"), (240, 88)),
];
