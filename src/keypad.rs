//! Keypad implementation, port of the Arduino lib.

use hal_base::prelude::*;
use hal_base::digital::*;
use bit_field::*;

pub struct Keypad<'k, I, O> {
    keymap: &'k [&'k [char]],
    rows: &'k mut [I],
    cols: &'k mut [O],
    last_scan: u32,
    debtime: u32,
    active: u64,
    keys: [Key; 16],
}

#[derive(Clone, Copy, PartialEq)]
pub enum KeyState {
    Idle,
    Pressed,
    Released,
}

#[derive(new, Clone, Copy)]
pub struct Key {
    kchar: char,
    #[new(value = "-1")]
    kcode: i16,
    #[new(value = "KeyState::Idle")]
    kstate: KeyState,
    #[new(value = "false")]
    changed: bool,
}

impl Key {
}

const NO_KEY: char = '\0';

impl<'k, I: InputPin, O: OutputPin> Keypad<'k, I, O> {
    pub fn new(keymap: &'k [&'k [char]], rows: &'k mut [I], cols: &'k mut [O]) -> Self {
        assert!(keymap.len() == rows.len());
        for kmrow in keymap {
            assert!(kmrow.len() == cols.len());
        }
        Keypad { keymap, rows, cols, last_scan: 0, debtime: 10, active: 0,
                 keys: [Key::new(NO_KEY); 16] }
    }

    pub fn get_key(&mut self, time: u32) -> char {
        let active = if self.last_scan + self.debtime >= time {
            self.last_scan = time;
            self.scan_keys();
            self.update_list()
        } else {
            false
        };
        if active && self.keys[0].changed && self.keys[0].kstate == KeyState::Pressed {
            self.keys[0].kchar
        } else {
            NO_KEY
        }
    }

    pub fn update(&mut self, time: u32) -> usize {
        if self.last_scan + self.debtime >= time {
            self.last_scan = time;
            self.scan_keys();
            self.update_list();
        }
        self.keys.iter().filter(|k| k.kchar != NO_KEY).count()
    }

    pub fn key_pressed(&mut self, key: char) -> bool {
        self.keys.iter().find(|k| k.kchar == key && k.kstate == KeyState::Pressed).is_some()
    }

    pub fn key_state_changed(&mut self, key: char) -> Option<bool> {
        self.keys.iter().find(|k| k.kchar == key && k.changed)
                        .map(|k| k.kstate == KeyState::Pressed)
    }

    fn scan_keys(&mut self) {
        self.active = 0;
        let nrows = self.rows.len();
        for (i, col) in self.cols.iter_mut().enumerate() {
            col.set_low();
            for (j, row) in self.rows.iter_mut().enumerate() {
                if row.is_low() {
                    self.active.set_bit(i*nrows + j, true);
                }
            }
            col.set_high();
        }
    }

    fn update_list(&mut self) -> bool {
        for key in &mut self.keys {
            if key.kstate == KeyState::Idle {
                *key = Key::new(NO_KEY);
            }
        }

        let nrows = self.rows.len();
        for i in 0..self.cols.len() {
            for j in 0..self.rows.len() {
                let kcode = i*nrows + j;
                let is_active = self.active.get_bit(kcode);
                match self.keys.iter_mut().find(|k| k.kcode == kcode as i16) {
                    None => if is_active {
                        if let Some(k) = self.keys.iter_mut().find(|k| k.kchar == NO_KEY) {
                            k.kchar = self.keymap[j][i];
                            k.kcode = kcode as i16;
                            k.kstate = KeyState::Idle;
                            Self::next_state(k, is_active)
                        }
                    },
                    Some(k) => Self::next_state(k, is_active)
                }
            }
        }

        self.keys.iter().any(|k| k.changed)
    }

    fn next_state(key: &mut Key, is_active: bool) {
        key.changed = false;
        match key.kstate {
            KeyState::Idle => if is_active {
                key.kstate = KeyState::Pressed;
                key.changed = true;
            },
            KeyState::Pressed => if !is_active {
                key.kstate = KeyState::Released;
                key.changed = true;
            },
            KeyState::Released => {
                key.kstate = KeyState::Idle;
                key.changed = true;
            }
        }
    }
}
