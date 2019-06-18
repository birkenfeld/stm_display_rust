//! Implementation of the "special" override mode of the display.

use cortex_m::asm;
use crate::{wait_touch, recv_uart, interface, framebuf};
use crate::framebuf::{MEDIUMFONT as FONT, BLACK_ON_WHITE, RED_ON_WHITE};

pub const ACTIVATION: &[u16] = &[1, 1, 2, 2, 0, 3, 0, 3];

const PXE_SCRIPT: &[u8] = b"http://ictrlfs.ictrl.frm2/public/echo.pxe";


pub fn run(disp: &mut interface::DisplayState) {
    let was_gfx = disp.is_graphics();
    let (gfx, con) = disp.split();

    gfx.activate();
    gfx.clear(15);

    gfx.rect_outline(8, 8, 116, 120, 0);
    gfx.text(FONT, 20, 56, b"Reset APU", BLACK_ON_WHITE);
    gfx.rect_outline(124, 8, 236, 120, 0);
    gfx.text(FONT, 140, 56, b"Reinstall", RED_ON_WHITE);
    gfx.rect_outline(244, 8, 356, 120, 0);
    gfx.text(FONT, 260, 56, b"Explode", BLACK_ON_WHITE);
    gfx.rect_outline(364, 8, 472, 120, 0);
    gfx.text(FONT, 380, 56, b"Cancel", BLACK_ON_WHITE);

    // TODO: clear all incoming uart data after selection is made?

    let action: &[u8] = match wait_touch().0 {
        x if x < 120 => b"Resetting",
        x if x < 240 => b"Reinstalling",
        x => {
            if x < 360 {
                explode(gfx);
            }
            if was_gfx {
                gfx.clear(0);
            } else {
                con.activate();
            }
            return;
        }
    };

    gfx.clear(15);
    gfx.text(FONT, 20, 30, action, RED_ON_WHITE);

    let mut uart_ring = wheelbuf::WheelBuf::new([0u8; 8]);
    loop {
        let _ = uart_ring.push(recv_uart());
        if uart_ring.iter().eq(b"PXE boot") {
            // activate the "press N for PXE boot" option
            gfx.text(FONT, 20, 80, b"PXE", BLACK_ON_WHITE);
            con.write_to_host(b"N");
        } else if uart_ring.iter().eq(b"autoboot") {
            // go up to the "shell" menu item, then start dhcp
            gfx.text(FONT, 20+4*8, 80, b"DHCP", &[15, 7, 8, 0]);
            con.write_to_host(b"\x1b[A\ndhcp\n");
        } else if uart_ring.iter().take(3).eq(b" ok") {
            // run our custom pxe script
            gfx.text(FONT, 20+9*8, 80, b"IMG", &[15, 7, 8, 0]);
            for &ch in b"imgexec ".iter().chain(PXE_SCRIPT).chain(b"\n") {
                // firmware keyboard buffer is only ~15 chars, need to wait...
                con.write_to_host(&[ch]);
                recv_uart();
            }
            // PXE is booting, back to normal mode to let the user know
            // what's happening
            con.activate();
            return;
        }
    }
}


fn explode(gfx: &mut framebuf::FrameBuffer) {
    for i in 0..=240 {
        if i >= 120 {
            let j = i - 120;
            gfx.rect(240 - j, 64 - j.min(64), 240 + j + 1, 64 + j.min(64), 11);
            gfx.rect(240 - i, 0, 240 - j, 128, 1);
            gfx.rect(240 + j + 1, 0, 240 + i + 1, 128, 1);
        } else {
            gfx.rect(240 - i, 64 - i.min(64), 240 + i + 1, 64 + i.min(64), 1);
        }
        asm::delay(1000000);
    }
    for _ in 0..5 {
        asm::delay(20000000);
        gfx.rect(190, 44, 291, 85, 11);
        asm::delay(20000000);
        gfx.text(&framebuf::FONTS[2], 190, 44, b"BOOM?", &[11, 214, 202, 9]);
    }
    for _ in 0..10 {
        asm::delay(20000000);
    }
}
