//! State machine abstraction for the main program.

use core::fmt::Write as FmtWrite;
use core::marker::PhantomData;
use btoi::btoi;
use phytron::{Phytron, Axis};
use keypad::Keypad;
use timer::Millis;
use framebuf::{FrameBuffer, FONTS};
use heapless::String;
use heapless::consts::*;
use hal_base::serial::Write;
use hal_base::digital::*;

#[derive(new)]
pub struct Resources<'a, W, I, O> {
    disp: FrameBuffer,
    phyt: Phytron<'a, W>,
    keys: Keypad<'a, I, O>,
    millis: Millis
}

#[derive(PartialEq)]
enum Mode {
    Auto,
    Manual,
    Free,
}

enum State {
    Start(Mode, String<U16>),
    ManStepping(u32, u32),
    AutoYLimits(u32),
    AutoCutDir(u32, i32),
    AutoConfirm(u32, i32, bool),
    AutoStepping(u32, i32, bool),
    FreePos,
}

impl State {
    fn init<'a, W: Write<u8>, I, O>(&mut self, res: &mut Resources<'a, W, I, O>) {
        match self {
            State::Start(..) => {
                res.disp.clear(0);
                res.disp.set_cursor(0, 2);
                writeln!(res.disp, "gear macchina omega");
                res.disp.set_cursor(0, 3);
                writeln!(res.disp, "gia poly lazy");
                self.redraw(res);
            }
            State::AutoYLimits(..) => {
                res.disp.clear(0);
                res.phyt.drive_to_init(Axis::Y);
            }
            _ => ()
        }
    }

    fn redraw<'a, W, I, O>(&mut self, res: &mut Resources<'a, W, I, O>) {
        match self {
            State::Start(mode, teeth) => {
                let text = match mode {
                    Mode::Auto   => "(auto)             ",
                    Mode::Manual => "(manual)           ",
                    Mode::Free   => "(free positioning) ",
                };
                res.disp.set_cursor(0, 0);
                writeln!(res.disp, "{}", text);
                res.disp.set_cursor(0, 1);
                //writeln!(res.disp, "teeth: {}", teeth)
            }
            _ => ()
        }
    }

    fn iter<'a, W: Write<u8>, I: InputPin, O: OutputPin>(
        &mut self, res: &mut Resources<'a, W, I, O>) -> Option<Self>
    {
        match self {
            State::Start(mode, teeth) => {
                match res.keys.get_key(res.millis.get()) {
                    'f' => {
                        *mode = match mode {
                            Mode::Auto => Mode::Manual,
                            Mode::Manual => Mode::Free,
                            Mode::Free => Mode::Auto,
                        };
                        self.redraw(res);
                    }
                    'F' => {
                        res.phyt.drive_to_init(Axis::Y);
                        self.redraw(res);
                    }
                    k @ ('0' ... '9') => {
                        teeth.push(k);
                        self.redraw(res);
                    }
                    'e' => {
                        teeth.clear();
                        self.redraw(res);
                    }
                    'g' => return match mode {
                        Mode::Free => Some(State::FreePos),
                        Mode::Auto => btoi(teeth.as_bytes()).ok().map(State::AutoYLimits),
                        Mode::Manual => btoi(teeth.as_bytes()).ok().map(|n| State::ManStepping(n, 0)),
                    },
                    _ => ()
                }
            }
            State::AutoYLimits(nteeth) => {
                res.keys.update(res.millis.get());
                match res.keys.key_state_changed('>') {
                    Some(true)  => res.phyt.move_cont(Axis::Y, false),
                    Some(false) => res.phyt.stop(Axis::Y),
                    _ => ()
                }
                match res.keys.key_state_changed('<') {
                    Some(true)  => res.phyt.move_cont(Axis::Y, true),
                    Some(false) => res.phyt.stop(Axis::Y),
                    _ => ()
                }
                match res.keys.get_key(res.millis.get()) {
                    'e' => {
                        res.phyt.stop(Axis::Y);
                        return Some(State::Start(Mode::Auto, String::new()));
                    }
                    'F' => {
                        res.phyt.drive_to_init(Axis::Y);
                    }
                    'g' => {
                        let pos = res.phyt.get_pos(Axis::Y);
                        return Some(State::AutoCutDir(*nteeth, pos));
                    }
                    _ => ()
                }
            }
            State::AutoCutDir(nteeth, init_pos) => {
                match res.keys.get_key(res.millis.get()) {
                    '>' => return Some(State::AutoConfirm(*nteeth, *init_pos, true)),
                    '<' => {
                        res.phyt.drive_to_init(Axis::Y);
                        return Some(State::AutoConfirm(*nteeth, *init_pos, false));
                    }
                    'e' => return Some(State::Start(Mode::Auto, String::new())),
                    _ => ()
                }
            }
            State::AutoConfirm(nteeth, init_pos, backw) => {
                match res.keys.get_key(res.millis.get()) {
                    'g' => return Some(State::AutoStepping(*nteeth, *init_pos, *backw)),
                    'e' => return Some(State::Start(Mode::Auto, String::new())),
                    _ => ()
                }
            }
            State::AutoStepping(nteeth, init_pos, backw) => {
                
            }
            _ => ()
        }
        None
    }
}

pub fn run<'a, W: Write<u8>, I: InputPin, O: OutputPin>(res: &mut Resources<'a, W, I, O>) {
    let mut state = State::Start(Mode::Auto, String::new());
    loop {
        state.init(res);
        state.redraw(res);
        loop {
            if let Some(new) = state.iter(res) {
                state = new;
                break;
            }
        }
    }
}
