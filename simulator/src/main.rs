use std::cell::Cell;
use std::fs::File;
use std::io::{Read, Write};
use std::os::unix::io::{AsRawFd, FromRawFd};
use crossbeam_channel::{unbounded, Receiver};
use structopt::StructOpt;
use nix::{pty, fcntl::OFlag};

#[derive(StructOpt)]
#[structopt(about = "Box display simulator.")]
pub struct Options {
    #[structopt(short="x", help="Scale display up by a factor of 2 or 4")]
    scale: Option<u8>,
}

fn prepare_tty() -> nix::Result<(Receiver<u8>, File)> {
    let master = pty::posix_openpt(OFlag::O_RDWR | OFlag::O_NOCTTY)?;
    pty::grantpt(&master)?;
    pty::unlockpt(&master)?;
    println!("Terminal open, connect clients to {}", pty::ptsname_r(&master)?);
    let fd = unsafe { File::from_raw_fd(nix::unistd::dup(master.as_raw_fd())?) };

    let (tx, rx) = unbounded();
    std::thread::spawn(move || {
        let fd = unsafe { File::from_raw_fd(master.as_raw_fd()) };
        for byte in fd.bytes() {
            if let Ok(byte) = byte {
                tx.send(byte).unwrap();
            } else {
                // something disconnected, wait for reconnect
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
        }
    });

    Ok((rx, fd))
}

struct WriteToHost {
    fd: File,
}

impl display::console::WriteToHost for WriteToHost {
    fn write_byte(&mut self, byte: u8) {
        let _ = self.fd.write(&[byte]);
    }
}

struct TouchHandler;

impl display::interface::TouchHandler for TouchHandler {
    type Event = (u16, u16);

    fn convert(&self, ev: (u16, u16)) -> (u16, u16) { ev }  // don't need to calibrate
    fn set_calib(&mut self, _: (u16, u16, u16, u16)) { }
    fn wait(&self) -> (u16, u16) { unimplemented!() }  // don't need this
}

struct FbImpl<'a> {
    is_console: bool,
    disp_switch: &'a Cell<bool>,
}

impl display::framebuf::FbImpl for FbImpl<'_> {
    fn fill_rect(&mut self, buf: &mut [u8], x1: u16, y1: u16, x2: u16, y2: u16, color: u8) {
        for y in y1..y2 {
            for x in x1..x2 {
                buf[(x + y * display::WIDTH) as usize] = color;
            }
        }
    }

    fn copy_rect(&mut self, buf: &mut [u8], x1: u16, y1: u16, x2: u16, y2: u16,
                 nx: u16, ny: u16) {
        let old = buf.to_vec();
        for iy in 0..ny {
            for ix in 0..nx {
                buf[(x2 + ix + (y2 + iy) * display::WIDTH) as usize] =
                    old[(x1 + ix + (y1 + iy) * display::WIDTH) as usize];
            }
        }
    }

    fn activate(&self, _: &mut [u8]) {
        self.disp_switch.set(self.is_console);
    }
}


fn main() {
    let args = Options::from_args();

    // open the pseudo-tty for clients to connect to
    let (rx, fd) = prepare_tty().expect("could not create pseudo tty");

    let scale = match args.scale {
        Some(2) => minifb::Scale::X2,
        Some(4) => minifb::Scale::X4,
        _ => minifb::Scale::X1,
    };

    // open the window
    let mut win = minifb::Window::new("Display", 480, 128, minifb::WindowOptions {
        scale, .. Default::default()
    }).expect("could not create window");

    // prepare color LUT (this is done in hardware on the STM)
    let lut = display::console::get_lut_colors().map(|(r, g, b)| {
        (r as u32) << 16 | (g as u32) << 8 | (b as u32)
    }).collect::<Vec<_>>();

    const WIDTH: usize = display::WIDTH as usize;
    const HEIGHT: usize = display::HEIGHT as usize;

    // prepare framebuffers
    let mut fb_graphics = vec![0; WIDTH*HEIGHT];
    let mut fb_console = vec![0; WIDTH*(HEIGHT + 8)];  // including extra row

    let mut fb_32bit = vec![0_u32; WIDTH*HEIGHT];  // actually displayed by minifb

    let console_active = Cell::new(true);

    let console = display::console::Console::new(
        display::framebuf::FrameBuffer::new(
            fb_console.as_mut_slice(),
            display::WIDTH, display::HEIGHT,
            FbImpl { is_console: true, disp_switch: &console_active }),
        WriteToHost { fd },
        (|_, _| ()) as fn(_, _)
    );
    let mut disp = display::interface::DisplayState::new(
        display::framebuf::FrameBuffer::new(
            fb_graphics.as_mut_slice(),
            display::WIDTH, display::HEIGHT,
            FbImpl { is_console: false, disp_switch: &console_active }),
        console,
        TouchHandler
    );

    let mut mouse_was_down = false;

    let mut iteration = 0u32;
    loop {
        // process quit conditions
        if !win.is_open() {
            println!("Window closed, exiting");
            return;
        }
        if win.is_key_down(minifb::Key::Escape) {
            return;
        }
        // process input from remote tty
        let mut change = false;
        while let Ok(ch) = rx.try_recv() {
            disp.process_byte(ch);
            change = true;
        }
        iteration = iteration.wrapping_add(1);
        if change || iteration % 20 == 0 {
            // check which framebuffer to display, and prepare the 32-bit buffer
            let fb = if console_active.get() {
                disp.console().buf()
            } else {
                disp.graphics().buf()
            };
            for (out, &color) in fb_32bit.iter_mut().zip(fb) {
                *out = lut[color as usize];
            }
            win.update_with_buffer(&fb_32bit, WIDTH, HEIGHT)
               .expect("could not update window");
        } else {
            win.update();
        }
        // process "touch" input by mouse
        let mouse_is_down = win.get_mouse_down(minifb::MouseButton::Left);
        if mouse_is_down && !mouse_was_down {
            if let Some((x, y)) = win.get_mouse_pos(minifb::MouseMode::Discard) {
                disp.process_touch((x as u16, y as u16));
            }
        }
        mouse_was_down = mouse_is_down;
        // aim for a framerate of 20Hz
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}
