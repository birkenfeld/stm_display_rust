//! Talking to the Phytron motor controller

use core::fmt::Write as FmtWrite;
use arm;
use hal_base::serial::Write;
use heapless::String;
use heapless::consts::*;

#[derive(Clone, Copy)]
pub enum Axis { X, Y }

impl Axis {
    fn byte(&self) -> u8 {
        match *self { Axis::X => b'X', Axis::Y => b'Y' }
    }
    fn chr(&self) -> char {
        match *self { Axis::X => 'X', Axis::Y => 'Y' }
    }
}

#[derive(new)]
pub struct Phytron<'c, W> {
    tx: W,
    rx: ::UartConsumer<'c>,
}

impl<'c, W: Write<u8>> Phytron<'c, W> {
    pub fn comm(&mut self, msg: &[u8], mut buf: Option<&mut [u8]>) -> usize {
        block!(self.tx.write(b'\x02'));
        block!(self.tx.write(b'0')); // phytron addr
        for &ch in msg {
            block!(self.tx.write(ch));
        }
        block!(self.tx.write(b'\x03'));
        hprintln!("written");

        let mut ix = 0;
        loop {
            if let Some(ch) = arm::interrupt::free(|_| self.rx.dequeue()) {
                hprintln!("ch: {:?}", ch);
                match ch {
                    b'\x03' => return ix,
                    b'\x15' => return ix, // XXX handle error
                    b'\x02' | b'\x06' => continue,
                    _ => {
                        if let Some(b) = &mut buf {
                            b[ix] = ch;
                        }
                        ix += 1;
                    }
                }
            }
        }
    }

    pub fn wait_for(&mut self, axis: Axis) {
        let mut buf = [0; 64];
        loop {
            // XXX: emergency stop

            self.comm(b"SE", Some(&mut buf));
            let index = match axis {
                Axis::X => 1,
                Axis::Y => 5
            };
            if btoi::btoi_radix::<u16>(&[buf[index]], 16).unwrap() & 1 != 0 {
                return;
            }
        }
    }

    pub fn drive_to_init(&mut self, axis: Axis) {
        let cmd = [axis.byte(), b'0', b'-'];
        self.comm(&cmd, None);
        self.wait_for(axis);
    }

    pub fn move_cont(&mut self, axis: Axis, backwd: bool) {
        let mut cmd: String<U64> = String::new();
        writeln!(cmd, "{}P14S{}", axis.chr(), 10); // TODO speed
        self.comm(cmd.as_bytes(), None);
        cmd.clear();
        writeln!(cmd, "{}L{}", axis.chr(), if backwd { '-' } else { '+' });
        self.comm(cmd.as_bytes(), None);
    }

    pub fn stop(&mut self, axis: Axis) {
        let cmd = [axis.byte(), b'S'];
        self.comm(&cmd, None);
    }

    pub fn get_pos(&mut self, axis: Axis) -> i32 {
        let mut buf = [0; 64];
        let cmd = [axis.byte(), b'P', b'1', b'9', b'R'];
        self.comm(&cmd, Some(&mut buf));
        btoi::btoi(&buf).unwrap()
    }

    pub fn move_relative(&mut self, axis: Axis, steps: u32) {
        // TODO
    }
}
