//! Implementation of the "special" override mode of the display.

use cortex_m::asm;
use embedded_hal::digital::v2::OutputPin;
use display::interface::TouchHandler;
use display::framebuf::{FONTS, MEDIUMFONT as FONT, BLACK_ON_WHITE, RED_ON_WHITE};
use crate::{DisplayState, Console, FrameBuffer, recv_uart, clear_uart};

pub const ACTIVATION: &[u16] = &[1, 1, 2, 2, 0, 3, 0, 3];

const PXE_SCRIPT: &[u8] = b"http://ictrlfs.ictrl.frm2/public/echo.pxe";


pub fn run<P: OutputPin>(disp: &mut DisplayState, reset_pin: &mut P) {
    let mut was_gfx = disp.is_graphics();
    let (gfx, con, touch) = disp.split();

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

    let x = touch.wait().0;
    // discard anything the APU sent while the menu was displayed
    clear_uart();

    if x < 240 {
        was_gfx = false;  // always start with console on reset
        reset_apu(reset_pin);
        if x > 120 {
            enter_netinstall(gfx, con);
        }
    } else if x < 360 {
        explode(gfx);
    }

    if was_gfx {
        gfx.clear(0);
    } else {
        con.activate();
    }
}


fn respond_to_prompt(con: &mut Console, prompt: &[u8], reply: impl IntoIterator<Item=&'static u8>) {
    let mut uart_ring = wheelbuf::WheelBuf::new([0u8; 8]);
    loop {
        let _ = uart_ring.push(recv_uart());
        if uart_ring.iter().eq(prompt) {
            // firmware keyboard buffer is only ~15 chars, need to send single
            // chars and wait for the echo back...
            for &ch in reply {
                con.write_to_host(&[ch]);
                recv_uart();
            }
            return;
        }
    }
}


fn enter_netinstall(gfx: &mut FrameBuffer, con: &mut Console) {
    gfx.clear(15);
    gfx.text(FONT, 20, 30, b"Rebooting for reinstall...", RED_ON_WHITE);

    respond_to_prompt(con, b"PXE boot", b"N");
    gfx.text(FONT, 20, 80, b"PXE", BLACK_ON_WHITE);
    respond_to_prompt(con, b"autoboot", b"\x1b[A\ndhcp\n");
    gfx.text(FONT, 20 + 4*8, 80, b"DHCP", BLACK_ON_WHITE);
    respond_to_prompt(con, b"..... ok", b"imgexec ".iter().chain(PXE_SCRIPT).chain(b"\n"));
    gfx.text(FONT, 20 + 9*8, 80, b"IMG", BLACK_ON_WHITE);

    // PXE is booting, back to normal mode to let the user know what's happening
}


fn reset_apu<P: OutputPin>(reset_pin: &mut P) {
    let _ = reset_pin.set_low();
    for _ in 0..50 {
        asm::delay(1000000);
    }
    let _ = reset_pin.set_high();
}


fn explode(gfx: &mut FrameBuffer) {
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
        gfx.text(&FONTS[2], 190, 44, b"BOOM?", &[11, 214, 202, 9]);
    }
    for _ in 0..10 {
        asm::delay(20000000);
    }
}
