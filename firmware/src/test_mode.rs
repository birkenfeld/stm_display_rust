//! Implementation of the hardware test mode of the display.

use embedded_hal::digital::v2::OutputPin;
use display::interface::TouchHandler;
use display::framebuf::{MEDIUMFONT as FONT, BLACK_ON_WHITE, RED_ON_WHITE, GREEN_ON_WHITE};
use crate::{stm, DisplayState, spiflash::SPIFlash, i2ceeprom::I2CEEprom};

const DATA: &[u8; 16] = b"\xff\xaa\x55\x00Test data\x00\x00\x00";

const P_X: u16 = 184;
const P_Y: u16 = 24;
const P_X2: u16 = P_X + 88;


pub fn run<T1, T2: OutputPin>(disp: &mut DisplayState, spi_flash: &mut SPIFlash<T1, T2>,
                              eeprom: &mut I2CEEprom) {
    let (gfx, _con, touch) = disp.split();

    // #1. Display
    gfx.clear(15);
    gfx.activate();

    gfx.line(0, 8, 480, 8, 0);
    gfx.text(FONT, 176, 0, b"Self test active", BLACK_ON_WHITE);
    gfx.text(FONT, 16, 32, b"Touch anywhere to cycle through colors.", BLACK_ON_WHITE);
    gfx.text(FONT, 16, 48, b"Make sure no pixel errors are present.", BLACK_ON_WHITE);
    touch.wait();

    // Set color palette for checking shorts between color pins.
    for i in 0..64 {
        write!(LTDC.layer1.clutwr: clutadd = i as u8, red = i*4, green = 0, blue = 0);
        write!(LTDC.layer1.clutwr: clutadd = i as u8 + 64, red = 0, green = i*4, blue = 0);
        write!(LTDC.layer1.clutwr: clutadd = i as u8 + 128, red = 0, green = 0, blue = i*4);
        write!(LTDC.layer1.clutwr: clutadd = i as u8 + 192, red = i*4, green = i*4, blue = i*4);
    }

    gfx.clear(0);
    for i in 0..64 {
        gfx.rect(i*7, 0,  i*7+6, 31,  i as u8);
        gfx.rect(i*7, 32, i*7+6, 63,  i as u8+64);
        gfx.rect(i*7, 64, i*7+6, 95,  i as u8+128);
        gfx.rect(i*7, 96, i*7+6, 127, i as u8+192);
    }
    touch.wait();

    // Set default color palette
    for (i, (r, g, b)) in display::console::get_lut_colors().enumerate() {
        write!(LTDC.layer1.clutwr: clutadd = i as u8, red = r, green = g, blue = b);
    }

    gfx.clear(196);
    touch.wait();
    gfx.clear(46);
    touch.wait();
    gfx.clear(21);
    touch.wait();
    gfx.clear(15);
    touch.wait();

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

    if let Err(_) = eeprom.write_at_addr(0x1000, DATA) {
        gfx.text(FONT, P_X2, P_Y+16, b"FAIL", RED_ON_WHITE);
    } else {
        let mut buf = [0; 16];
        if let Err(_) = eeprom.read_at_addr(0x1000, &mut buf) {
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
    // SKIPPED.
    // gfx.text(FONT, P_X, P_Y+32, b"UART......", BLACK_ON_WHITE);

    // let mut failed = false;
    // for &ch in DATA {
    //     con.write_to_host(&[ch]);
    //     if recv_uart() != ch {
    //         gfx.text(FONT, P_X2, P_Y+32, b"FAIL", RED_ON_WHITE);
    //         failed = true;
    //         break;
    //     }
    // }
    // if !failed {
    //     gfx.text(FONT, P_X2, P_Y+32, b"OK", GREEN_ON_WHITE);
    // }

    // #5. Touch

    /* For touch calibration */
    // gfx.clear(15);
    // loop {
    //     let (x, _) = touch.wait();
    //     gfx.rect(x - 1, 0, x + 1, 127, 4);
    //     touch.wait();
    //     gfx.clear(15);
    // }

    gfx.text(FONT, P_X, P_Y+48, b"Touch.....", BLACK_ON_WHITE);
    gfx.rect_outline(8, 20, 120, 120, 0);
    gfx.text(FONT, 20, 56, b"Touch here", BLACK_ON_WHITE);
    while touch.wait().0 > 120 {}
    gfx.rect(8, 20, 120, 120, 15);

    gfx.rect_outline(352, 20, 472, 120, 0);
    gfx.text(FONT, 364, 56, b"Touch here", BLACK_ON_WHITE);
    while touch.wait().0 < 364 {}
    gfx.rect(352, 20, 472, 120, 15);
    gfx.text(FONT, P_X2, P_Y+48, b"OK", GREEN_ON_WHITE);

    gfx.text(FONT, 16, 96, b"Touch anywhere to exit self test mode.", BLACK_ON_WHITE);
    touch.wait();
}
