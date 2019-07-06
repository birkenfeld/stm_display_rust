//! Icon definitions.

const MLZLOGO: &[u8] = include_bytes!("logo_mlz.dat");
const MLZLOGO_SIZE: (u16, u16) = (240, 88);
// const MLZ_COLORS: [u8; 4] = [60, 104, 188, 255];

// const ARROW_UP: &[u8] = &[
//     0b10000000, 0b00000001,
//     0b11000000, 0b00000011,
//     0b11100000, 0b00000111,
//     0b11110000, 0b00001111,
//     0b11111000, 0b00011111,
//     0b11111100, 0b00111111,
//     0b11111110, 0b01111111,
//     0b11111111, 0b11111111
// ];
// const ARROW_SIZE: (u16, u16) = (16, 8);

pub const ICONS: &[(&[u8], (u16, u16))] = &[
    (MLZLOGO, MLZLOGO_SIZE),
];
