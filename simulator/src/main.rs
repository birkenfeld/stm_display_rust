use std::cell::Cell;
use std::fs::File;
use std::io::{Read, Write};
use std::os::unix::io::{AsRawFd, FromRawFd};
use crossbeam_channel::{unbounded, Receiver};
use structopt::StructOpt;
use nix::{pty, fcntl::OFlag};
use display::{WIDTH, HEIGHT};

#[derive(StructOpt)]
#[structopt(about = "Box display simulator.")]
pub struct Options {
    #[structopt(short="x", help="Scale display up by a factor of 2")]
    scale_x2: bool,
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
            }
        }
    });

    Ok((rx, fd))
}

struct WriteToHost(File);

impl display::console::WriteToHost for WriteToHost {
    fn write_byte(&mut self, byte: u8) {
        let _ = self.0.write(&[byte]);
    }
}

struct TouchHandler;

impl display::interface::TouchHandler for TouchHandler {
    type Event = (u16, u16);

    fn convert(&self, ev: (u16, u16)) -> (u16, u16) { ev }  // don't need to calibrate
    fn set_calib(&mut self, _: (u16, u16, u16, u16)) { }
    fn wait(&self) -> (u16, u16) { unimplemented!() }  // don't need this
}

struct FbImpl<'a>(bool, &'a Cell<bool>);

impl display::framebuf::FbImpl for FbImpl<'_> {
    fn fill_rect(&mut self, buf: &mut [u8], x1: u16, y1: u16, x2: u16, y2: u16, color: u8) {
        for y in y1..y2 {
            for x in x1..x2 {
                buf[(x + y * WIDTH) as usize] = color;
            }
        }
    }

    fn copy_rect(&mut self, buf: &mut [u8], x1: u16, y1: u16, x2: u16, y2: u16, nx: u16, ny: u16) {
        // TODO handle cases other than shifting to the top left
        for iy in 0..ny {
            for ix in 0..nx {
                buf[(x2 + ix + (y2 + iy) * WIDTH) as usize] =
                    buf[(x1 + ix + (y1 + iy) * WIDTH) as usize];
            }
        }
    }

    fn activate(&self, _: &mut [u8]) {
        self.1.set(self.0);
    }
}


fn main() {
    let args = Options::from_args();

    // open the pseudo-tty for clients to connect to
    let (rx, fd) = prepare_tty().expect("could not create pseudo tty");

    // open the window
    let mut win = minifb::Window::new("Display", 480, 128, minifb::WindowOptions {
        scale: if args.scale_x2 { minifb::Scale::X2 } else { minifb::Scale::X1 },
        .. Default::default()
    }).expect("could not create window");

    // prepare color LUT (this is done in hardware on the STM)
    let lut = display::console::get_lut_colors().map(|(r, g, b)| {
        (r as u32) << 16 | (g as u32) << 8 | (b as u32)
    }).collect::<Vec<_>>();

    // prepare framebuffers
    let mut fb_graphics = vec![0; (WIDTH*HEIGHT) as usize];
    let mut fb_console = vec![0; (WIDTH*(HEIGHT + 8)) as usize];  // including extra row
    let mut fb_32bit = vec![0_u32; (WIDTH*HEIGHT) as usize];

    let console_active = Cell::new(true);

    let console = display::console::Console::new(
        display::framebuf::FrameBuffer::new(
            &mut fb_console, WIDTH, HEIGHT, FbImpl(true, &console_active)),
        WriteToHost(fd),
        |_, _| ()  // ignore cursor
    );
    let mut disp = display::interface::DisplayState::new(
        display::framebuf::FrameBuffer::new(
            &mut fb_graphics, WIDTH, HEIGHT, FbImpl(false, &console_active)),
        console,
        TouchHandler
    );

    let mut mouse_was_down = false;

    loop {
        // process input from remote tty
        let mut need_update = false;
        while let Ok(ch) = rx.try_recv() {
            if let display::interface::Action::Reset = disp.process_byte(ch) {
                println!("Would reset the display!");
            }
            need_update = true;
        }
        // process "touch" input by mouse
        let mouse_is_down = win.get_mouse_down(minifb::MouseButton::Left);
        if mouse_is_down && !mouse_was_down {
            if let Some((x, y)) = win.get_mouse_pos(minifb::MouseMode::Discard) {
                disp.process_touch((x as u16, y as u16));
                need_update = true;
            }
        }
        mouse_was_down = mouse_is_down;
        // update the framebuffer if something might have changed
        if need_update {
            // check which framebuffer to display, and prepare the 32-bit buffer
            let fb = if console_active.get() { disp.console().buf() } else { disp.graphics().buf() };
            for (out, &color) in fb_32bit.iter_mut().zip(fb) {
                *out = lut[color as usize];
            }
            win.update_with_buffer(&fb_32bit).expect("could not update window");
        } else {
            win.update();
        }
        // process quit conditions
        if !win.is_open() {
            println!("Window closed, exiting");
            return;
        }
        if win.is_key_down(minifb::Key::Escape) {
            return;
        }
        // aim for a framerate of 50Hz
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}
