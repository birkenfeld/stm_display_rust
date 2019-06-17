//! Implementation of the "special" override mode of the display.

use crate::{wait_touch, recv_uart, interface};
use crate::framebuf::{MEDIUMFONT as FONT, BLACK_ON_WHITE, RED_ON_WHITE};

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

    let action: &[u8] = match wait_touch().0 {
        x if x < 106 => b"Resetting",
        x if x < 162 => b"Reinstalling",
        _ => {
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
