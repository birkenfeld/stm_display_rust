//! Implementation of the hardware test mode of the display.

use embedded_hal::digital::v2::OutputPin;
use crate::{wait_touch, recv_uart, interface, spiflash, i2ceeprom};
use crate::framebuf::{MEDIUMFONT as FONT, BLACK_ON_WHITE, RED_ON_WHITE, GREEN_ON_WHITE};

const DATA: &[u8; 16] = b"\xff\xaa\x55\x00Test data\x00\x00\x00";

const P_X: u16 = 184;
const P_Y: u16 = 24;
const P_X2: u16 = P_X + 88;


pub fn run<T1, T2: OutputPin>(disp: &mut interface::DisplayState,
                              spi_flash: &mut spiflash::SPIFlash<T1, T2>,
                              eeprom: &mut i2ceeprom::I2CEEprom) {
    let (gfx, con) = disp.split();

    // #1. Display
    gfx.clear(15);
    gfx.activate();
    gfx.line(0, 8, 480, 8, 0);
    gfx.text(FONT, 176, 0, b"Self test active", BLACK_ON_WHITE);
    gfx.text(FONT, 16, 32, b"Touch anywhere to cycle through colors.", BLACK_ON_WHITE);
    gfx.text(FONT, 16, 48, b"Make sure no pixel errors are present.", BLACK_ON_WHITE);
    wait_touch();

    gfx.clear(1);
    wait_touch();
    gfx.clear(2);
    wait_touch();
    gfx.clear(4);
    wait_touch();
    gfx.clear(15);
    wait_touch();

    // #2. Flash memory
    gfx.clear(15);
    gfx.line(0, 8, 480, 8, 0);
    gfx.text(FONT, 176, 0, b"Self test active", BLACK_ON_WHITE);

    gfx.text(FONT, P_X, P_Y, b"Flash.....", BLACK_ON_WHITE);

    spi_flash.erase_sector(0);
    spi_flash.write_bulk(0x100, DATA);
    if spi_flash.read(0x100, DATA.len()).iter().eq(DATA) {
        gfx.text(FONT, P_X2, P_Y, b"OK", GREEN_ON_WHITE);
    } else {
        gfx.text(FONT, P_X2, P_Y, b"FAIL", RED_ON_WHITE);
    }

    // #3. EEPROM
    gfx.text(FONT, P_X, P_Y+16, b"E\xfdPROM....", BLACK_ON_WHITE);

    if let Err(_) = eeprom.write_at_addr(128, DATA) {
        gfx.text(FONT, P_X2, P_Y+16, b"FAIL", RED_ON_WHITE);
    } else {
        let mut buf = [0; 16];
        if let Err(_) = eeprom.read_at_addr(128, &mut buf) {
            gfx.text(FONT, P_X2, P_Y+16, b"FAIL", RED_ON_WHITE);
        } else {
            if &buf != DATA {
                gfx.text(FONT, P_X2, P_Y+16, b"FAIL", RED_ON_WHITE);
            } else {
                gfx.text(FONT, P_X2, P_Y+16, b"OK", GREEN_ON_WHITE);
            }
        }
    }

    // #4. UART
    gfx.text(FONT, P_X, P_Y+32, b"UART......", BLACK_ON_WHITE);

    let mut failed = false;
    for &ch in DATA {
        con.write_to_host(&[ch]);
        if recv_uart() != ch {
            gfx.text(FONT, P_X2, P_Y+32, b"FAIL", RED_ON_WHITE);
            failed = true;
            break;
        }
    }
    if !failed {
        gfx.text(FONT, P_X2, P_Y+32, b"OK", GREEN_ON_WHITE);
    }

    // #5. Touch
    gfx.text(FONT, P_X, P_Y+48, b"Touch.....", BLACK_ON_WHITE);
    gfx.rect_outline(8, 20, 120, 120, 0);
    gfx.text(FONT, 20, 56, b"Touch here", BLACK_ON_WHITE);
    while wait_touch().0 > 106 {}
    gfx.rect(8, 20, 121, 121, 15);

    gfx.rect_outline(352, 20, 472, 120, 0);
    gfx.text(FONT, 364, 56, b"Touch here", BLACK_ON_WHITE);
    while wait_touch().0 < 218 {}
    gfx.rect(352, 20, 473, 121, 15);
    gfx.text(FONT, P_X2, P_Y+48, b"OK", GREEN_ON_WHITE);

    gfx.text(FONT, 16, 96, b"Touch anywhere to exit self test mode.", BLACK_ON_WHITE);
    wait_touch();
}
