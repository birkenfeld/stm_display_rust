//! Icon definitions.

const MLZLOGO: &[u8] = include_bytes!("logo_mlz.dat");
const MLZLOGO_SIZE: (u16, u16) = (240, 88);
// const MLZ_COLORS: [u8; 4] = [60, 104, 188, 255];

const PSILOGO: &[u8] = include_bytes!("logo_psi.dat");
const PSILOGO_SIZE: (u16, u16) = (244, 88);

pub const ICONS: &[(&[u8], (u16, u16))] = &[
    (MLZLOGO, MLZLOGO_SIZE),
    (PSILOGO, PSILOGO_SIZE),
];
