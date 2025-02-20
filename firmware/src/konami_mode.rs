//! Implementation of the "special" override mode of the display.

use stm32f4xx_hal::hal::digital::OutputPin;
use display::interface::TouchHandler;
use display::framebuf::{MEDIUMFONT as FONT, BLACK_ON_WHITE, RED_ON_WHITE};
use crate::{DisplayState, Console, FrameBuffer, recv_uart, clear_uart, reset_apu,
            TEST_MODE};

pub const ACTIVATION: &[u16] = &[1, 1, 2, 2, 0, 3, 0, 3];

const PXE_SCRIPT: &[u8] = b"http://pxe.boxes.frm2.tum.de/box.pxe";
const PXE_SCRIPT_WIPE: &[u8] = b"http://pxe.boxes.frm2.tum.de/box_wipe.pxe";


pub fn run<P: OutputPin>(disp: &mut DisplayState, reset_pin: &mut P, preset_opt: Option<u16>) {
    let mut was_gfx = disp.is_graphics();
    let (gfx, con, touch) = disp.split();

    gfx.activate();
    gfx.clear(15);

    let x = if let Some(opt) = preset_opt {
        opt*120 + 10
    } else {
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
        x
    };

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


fn slow_write(con: &mut Console, text: impl IntoIterator<Item=&'static u8>) {
    // firmware keyboard buffer is only ~15 chars, need to send single
    // chars and wait for the echo back...
    for &ch in text {
        con.write_to_host(&[ch]);
        con.process_char(recv_uart());
    }
}


fn respond_to_prompt(con: &mut Console, prompt: &[u8], outbuf: &mut [u8],
                     reply: impl IntoIterator<Item=&'static u8>,
                     retry_prompt: bool, check_errors: bool) -> bool {
    let mut i = 0;
    let mut uart_ring = wheelbuf::WheelBuf::new([0u8; 16]);
    loop {
        let ch = recv_uart();
        con.process_char(ch);
        if i < outbuf.len() {
            outbuf[i] = ch;
            i += 1;
        }
        uart_ring.push(ch);
        if uart_ring.iter().skip(16 - prompt.len()).eq(prompt) {
            slow_write(con, reply);
            return true;
        } else if check_errors && uart_ring.iter().eq(b"https://ipxe.org") {
            for _ in 0..18 {
                // slurp until end of message: /abcdefgh)\r\niPXE>_
                con.process_char(recv_uart());
            }
            return false;
        } else if retry_prompt && uart_ring.iter().skip(14).eq(b"> ") {
            // sometimes the first iPXE> prompt is interrupted by ANSI
            // cursor movement, retry
            slow_write(con, b"\n");
        }
    }
}


fn enter_netinstall(gfx: &mut FrameBuffer, con: &mut Console, wipe: bool) {
    if TEST_MODE {
        // let us see directly what's going on
        con.activate();
    }
    gfx.clear(15);
    gfx.text(FONT, 20, 25, b"Rebooting for reinstall...", RED_ON_WHITE);

    respond_to_prompt(con, b"PXE boot", &mut [], b"N", false, false);
    gfx.text(FONT, 20, 85, b"PXE", BLACK_ON_WHITE);
    respond_to_prompt(con, b"autoboot", &mut [], b"\x1b[A\n", false, false);
    respond_to_prompt(con, b"iPXE> ", &mut [], b"ifstat net0\n", true, false);
    let mut outbuf = [0; 24];
    respond_to_prompt(con, b"iPXE> ", &mut outbuf, b"ifconf -c dhcp net0\n", false, false);
    gfx.text(FONT, 20, 55, &outbuf[1..], BLACK_ON_WHITE);
    gfx.text(FONT, 20 + 4*8, 85, b"DHCP", BLACK_ON_WHITE);

    loop {
        let exec_cmd = b"imgexec ".iter().chain(
            if wipe { PXE_SCRIPT_WIPE } else { PXE_SCRIPT }).chain(b"\n");
        if respond_to_prompt(con, b".. ok\r\niPXE> ", &mut [], exec_cmd, false, true) {
            gfx.text(FONT, 20 + 9*8, 85, b"IMG", BLACK_ON_WHITE);
            return;
            // PXE is booting, back to normal mode to let the user know what's happening
        }

        // DHCP got a timeout, try again
        slow_write(con, b"ifconf -c dhcp net0\n");
    }
}
