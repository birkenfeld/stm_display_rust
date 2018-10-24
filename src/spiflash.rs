//! Communication with the SPI flash using DMA.
#![allow(unused)]

use stm;
use hal_base::digital::OutputPin;

#[link_section = ".sram1bss"]
static mut DMABUF: [u8; 1029] = [0; 1029];

const OP_READ:           u8 = 0x03;
const OP_READ_HS:        u8 = 0x0B;
const OP_ERASE_4K:       u8 = 0x20;
const OP_ERASE_32K:      u8 = 0x52;
const OP_ERASE_64K:      u8 = 0xD8;
const OP_ERASE_ALL:      u8 = 0x60;
const OP_WRITE_BYTE:     u8 = 0x02;
const OP_WRITE_AAI:      u8 = 0xAD;
const OP_STATUS:         u8 = 0x05;
const OP_STATUS_WR_EN:   u8 = 0x50;
const OP_STATUS_WRITE:   u8 = 0x01;
const OP_WRITE_ENABLE:   u8 = 0x06;
const OP_WRITE_DISABLE:  u8 = 0x04;
const OP_READ_JEDEC:     u8 = 0x9F;

pub struct SPIFlash<SPI, CS> {
    #[allow(unused)]
    spi: SPI,
    cs: CS,
    wp_disabled: bool,
}

impl<SPI, CS: OutputPin> SPIFlash<SPI, CS> {
    pub fn new(spi: SPI, mut cs: CS) -> Self {
        cs.set_high();

        // Enable and reset the DMA controller.
        modif!(RCC.ahb1enr: dma1en = true);
        modif!(RCC.ahb1rstr: dma1rst = true);
        modif!(RCC.ahb1rstr: dma1rst = false);

        // Enable DMA transfers for SPI.
        modif!(SPI2.cr2: txdmaen = true);
        modif!(SPI2.cr2: rxdmaen = true);

        // Set up memory and peripheral addresses.
        // Stream 4: TX
        modif!(DMA1.s4cr: msize = 0, psize = 0,
               minc = true, pinc = false, dbm = false, ct = false, circ = false,
               dir = 0b01 /* m->p */, chsel = 0);
        modif!(DMA1.s4m0ar: m0a = &mut DMABUF as *const _ as u32);
        modif!(DMA1.s4par: pa = &(*stm::SPI2::ptr()).dr as *const _ as u32);
        // Stream 3: RX
        modif!(DMA1.s3cr: msize = 0, psize = 0,
               minc = true, pinc = false, dbm = false, ct = false, circ = false,
               dir = 0b00 /* p->m */, chsel = 0);
        modif!(DMA1.s3m0ar: m0a = &mut DMABUF as *const _ as u32);
        modif!(DMA1.s3par: pa = &(*stm::SPI2::ptr()).dr as *const _ as u32);

        let mut slf = Self { spi, cs, wp_disabled: false };

        // Check communication with Read JEDEC ID command.
        assert_eq!(slf.transfer(&[OP_READ_JEDEC], 3), &[0xBF, 0x25, 0x41]);

        slf
    }

    fn transfer<'a>(&'a mut self, inp: &[u8], outlen: usize) -> &'a [u8] {
        let ntotal = inp.len() + outlen;
        unsafe { DMABUF[..inp.len()].copy_from_slice(inp); }
        modif!(DMA1.s3ndtr: ndt = ntotal as u16);
        modif!(DMA1.s4ndtr: ndt = ntotal as u16);

        // Enable chip-select and start transfer.
        self.cs.set_low();
        modif!(DMA1.s3cr: en = true);
        modif!(DMA1.s4cr: en = true);

        // Wait for RX completion and reset flags.
        wait_for!(DMA1.lisr: tcif3);
        assert!(!readb!(DMA1.lisr: teif3));
        modif!(DMA1.lifcr: ctcif3 = true, cteif3 = true);
        modif!(DMA1.hifcr: ctcif4 = true, cteif4 = true);

        // Stop transfer.
        modif!(DMA1.s3cr: en = false);
        modif!(DMA1.s4cr: en = false);
        self.cs.set_high();

        unsafe { &DMABUF[inp.len()..ntotal] }
    }

    pub fn read<'a>(&'a mut self, addr: usize, len: usize) -> &'a [u8] {
        assert!(len <= 1024);
        assert!(addr + len < 0x1F_FFFF);
        self.transfer(&[OP_READ_HS, (addr >> 16) as u8, (addr >> 8) as u8, addr as u8, 0], len)
    }

    pub fn wait(&mut self) {
        while self.transfer(&[OP_STATUS], 1)[0] & 1 != 0 {}
    }

    fn disable_wp(&mut self) {
        if !self.wp_disabled {
            // disable write protection bits
            self.transfer(&[OP_STATUS_WR_EN], 0);
            self.transfer(&[OP_STATUS_WRITE, 0x00], 0);
            self.wp_disabled = true;
        }
    }

    pub fn write(&mut self, addr: usize, data: &[u8]) {
        assert!(data.len() >= 1);
        self.disable_wp();
        // write byte by byte
        for (i, &byte) in (addr..addr+data.len()).zip(data) {
            self.transfer(&[OP_WRITE_ENABLE], 0);
            self.transfer(&[OP_WRITE_BYTE, (i >> 16) as u8, (i >> 8) as u8, i as u8, byte], 0);
            self.wait(); // or wait 10us byte programming time
        }
    }

    pub fn write_bulk(&mut self, addr: usize, data: &[u8]) {
        assert_eq!(addr % 2, 0);
        assert_eq!(data.len() % 2, 0);
        assert!(data.len() >= 2);
        self.disable_wp();
        // start write process
        self.transfer(&[OP_WRITE_ENABLE], 0);
        // write first word
        self.transfer(&[OP_WRITE_AAI, (addr >> 16) as u8, (addr >> 8) as u8, addr as u8,
                        data[0], data[1]], 0);
        self.wait();
        for i in (2..data.len()).step_by(2) {
            self.transfer(&[OP_WRITE_AAI, data[i], data[i+1]], 0);
            self.wait();
        }
        // end write process
        self.transfer(&[OP_WRITE_DISABLE], 0);
    }

    pub fn erase_sector(&mut self, addr: usize) {
        assert_eq!(addr % 4096, 0);
        self.disable_wp();
        // start write process
        self.transfer(&[OP_WRITE_ENABLE], 0);
        self.transfer(&[OP_ERASE_4K, (addr >> 16) as u8, (addr >> 8) as u8, addr as u8], 0);
        self.wait();
    }
}
