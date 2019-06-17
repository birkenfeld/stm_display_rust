//! Communication with the I2C EEPROM using bit-banging.
#![allow(unused)]

use cortex_m::asm;
use hal::gpio::{gpioc, Output, OpenDrain};
use embedded_hal::digital::v2::{InputPin, OutputPin};

pub type Result<T> = core::result::Result<T, ()>;

pub struct I2CEEprom {
    scl: gpioc::PC4<Output<OpenDrain>>,
    sda: gpioc::PC5<Output<OpenDrain>>,
}

impl I2CEEprom {
    pub fn new(mut scl: gpioc::PC4<Output<OpenDrain>>, mut sda: gpioc::PC5<Output<OpenDrain>>) -> Self {
        scl.set_high();
        sda.set_high();
        Self { scl, sda }
    }

    fn delay(&self) { asm::delay(50); }

    fn start_cond(&mut self) {
        self.sda.set_high();
        self.delay();
        self.scl.set_high();
        self.delay();
        self.sda.set_low();
        self.delay();
        self.scl.set_low();
    }

    fn stop_cond(&mut self) {
        self.sda.set_low();
        self.delay();
        self.scl.set_high();
        self.delay();
        self.sda.set_high();
        self.delay();
    }

    fn write_bit(&mut self, bit: bool) {
        if bit { self.sda.set_high(); } else { self.sda.set_low(); }
        self.delay();
        self.scl.set_high();
        self.delay();
        self.scl.set_low();
    }

    fn read_bit(&mut self) -> bool {
        self.sda.set_high();
        self.delay();
        self.scl.set_high();
        self.delay();
        let bit = self.sda.is_high();
        self.scl.set_low();
        bit.unwrap()
    }

    fn write_byte(&mut self, mut byte: u8) -> Result<()> {
        // write 8 bits, starting with MSB
        for _ in 0..8 {
            self.write_bit(byte & 0x80 != 0);
            byte <<= 1;
        }
        if !self.read_bit() { Ok(()) } else { Err(()) }
    }

    fn read_byte(&mut self, ack: bool) -> u8 {
        // read 8 bits, starting with MSB
        let mut byte = 0;
        for _ in 0..8 {
            byte = (byte << 1) | self.read_bit() as u8;
        }
        self.write_bit(!ack);
        byte
    }

    fn write_devsel(&mut self, read: bool) -> Result<()> {
        self.write_byte(0b10100000 | if read { 1 } else { 0 })
    }

    pub fn read_at_current_addr(&mut self, buf: &mut [u8]) -> Result<()> {
        self.start_cond();
        self.write_devsel(true)?;
        let n = buf.len();
        for byte in &mut buf[..n-1] {
            *byte = self.read_byte(true);
        }
        buf[n-1] = self.read_byte(false);
        self.stop_cond();
        Ok(())
    }

    pub fn read_at_addr(&mut self, addr: usize, buf: &mut [u8]) -> Result<()> {
        assert!(addr + buf.len() <= 0x8000);
        self.start_cond();
        self.write_devsel(false)?;
        self.write_byte((addr >> 8) as u8)?;
        self.write_byte(addr as u8)?;
        self.read_at_current_addr(buf)
    }

    pub fn write_at_addr(&mut self, addr: usize, buf: &[u8]) -> Result<()> {
        // write within one 64-byte page
        assert!(addr + buf.len() <= 0x8000);
        assert!(addr & 0xfffc0 == (addr + buf.len() - 1) & 0xfffc0);
        self.start_cond();
        self.write_devsel(false)?;
        self.write_byte((addr >> 8) as u8)?;
        self.write_byte(addr as u8)?;
        for &byte in buf {
            self.write_byte(byte)?;
        }
        self.stop_cond();
        asm::delay(1_000_000);  // wait 5ms write time
        Ok(())
    }

    pub fn read_stored_entry<'a>(&mut self, len_addr: usize, data_addr: usize,
                                 data_buf: &'a mut [u8]) -> Result<&'a [u8]> {
        assert!(data_addr % 64 == 0);
        let mut len_buf = [0, 0];
        self.read_at_addr(len_addr, &mut len_buf)?;
        let len = (len_buf[0] as usize) | ((len_buf[1] as usize) << 8);
        // this excludes the unprogrammed case of 0xffff
        if len > 0 && len <= data_buf.len() {
            self.read_at_addr(data_addr, data_buf)?;
            Ok(data_buf)
        } else {
            Ok(&[])
        }
    }

    pub fn write_stored_entry(&mut self, len_addr: usize, data_addr: usize,
                              data_buf: &[u8]) -> Result<()> {
        assert!(data_addr % 64 == 0);
        let len_buf = [data_buf.len() as u8, (data_buf.len() >> 8) as u8];
        self.write_at_addr(len_addr, &len_buf)?;
        for (addr, chunk) in (data_addr..).step_by(64).zip(data_buf.chunks(64)) {
            self.write_at_addr(addr, chunk)?;
        }
        Ok(())
    }
}
