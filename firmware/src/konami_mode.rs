//! Implementation of the "special" override mode of the display.

use embedded_hal::digital::v2::OutputPin;
use display::interface::TouchHandler;
use display::framebuf::{MEDIUMFONT as FONT, BLACK_ON_WHITE, RED_ON_WHITE};
use crate::{DisplayState, Console, FrameBuffer, recv_uart, clear_uart, reset_apu,
            TEST_MODE};

pub const ACTIVATION: &[u16] = &[1, 1, 2, 2, 0, 3, 0, 3];

const PXE_SCRIPT: &[u8] = b"http://pxe.boxes.frm2.tum.de/box.pxe";
const PXE_SCRIPT_WIPE: &[u8] = b"http://pxe.boxes.frm2.tum.de/box_wipe.pxe";


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
    gfx.text(FONT, 260, 56, b"Wipe and", RED_ON_WHITE);
    gfx.text(FONT, 260, 70, b"reinstall", RED_ON_WHITE);
    gfx.rect_outline(364, 8, 472, 120, 0);
    gfx.text(FONT, 380, 56, b"Cancel", BLACK_ON_WHITE);

    let x = touch.wait().0;
    // discard anything the APU sent while the menu was displayed
    clear_uart();

    if x < 360 {
        was_gfx = false;  // always start with console on reset
        reset_apu(reset_pin);
        if x > 120 {
            enter_netinstall(gfx, con, x > 240);
        }
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
        let ch = recv_uart();
        con.process_char(ch);
        let _ = uart_ring.push(ch);
        if uart_ring.iter().eq(prompt) {
            // firmware keyboard buffer is only ~15 chars, need to send single
            // chars and wait for the echo back...
            for &ch in reply {
                con.write_to_host(&[ch]);
                con.process_char(recv_uart());
            }
            return;
        }
    }
}


fn enter_netinstall(gfx: &mut FrameBuffer, con: &mut Console, wipe: bool) {
    if TEST_MODE {
        // let us see directly what's going on
        con.activate();
    }
    gfx.clear(15);
    gfx.text(FONT, 20, 30, b"Rebooting for reinstall...", RED_ON_WHITE);

    respond_to_prompt(con, b"PXE boot", b"N");
    gfx.text(FONT, 20, 80, b"PXE", BLACK_ON_WHITE);
    respond_to_prompt(con, b"autoboot", b"\x1b[A\n");
    respond_to_prompt(con, b"2JiPXE> ", b"ifconf -c dhcp net0\n");
    gfx.text(FONT, 20 + 4*8, 80, b"DHCP", BLACK_ON_WHITE);
    // TODO: respond to "No configuration methods succeeded" with dhcp again
    respond_to_prompt(con, b"..... ok", b"imgexec ".iter().chain(
        if wipe { PXE_SCRIPT_WIPE } else { PXE_SCRIPT }
    ).chain(b"\n"));
    gfx.text(FONT, 20 + 9*8, 80, b"IMG", BLACK_ON_WHITE);

    // PXE is booting, back to normal mode to let the user know what's happening
}
