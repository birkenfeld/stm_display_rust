//! Prints "Hello, world!" on the OpenOCD console using semihosting
//!
//! ---

#![no_main]
#![no_std]
#![feature(asm)]
#![allow(unused)]

#[macro_use]
extern crate cortex_m_rt as rt;
extern crate cortex_m_semihosting as sh;
extern crate panic_semihosting;
#[macro_use]
extern crate nb;
extern crate embedded_hal as hal_base;
extern crate cortex_m as arm;
extern crate stm32f429 as stm;
extern crate stm32f429_hal as hal;

use core::fmt::Write;

use rt::ExceptionFrame;
use sh::hio;
use hal::time::*;
use hal::delay::Delay;
use hal::rcc::RccExt;
use hal::gpio::GpioExt;
use hal::flash::FlashExt;
use hal_base::prelude::*;

entry!(main);

const ILI9341_RESET: u8 = 0x01;
const ILI9341_SLEEP_OUT: u8 = 0x11;
const ILI9341_GAMMA: u8 = 0x26;
const ILI9341_DISPLAY_OFF: u8 = 0x28;
const ILI9341_DISPLAY_ON: u8 = 0x29;
const ILI9341_COLUMN_ADDR: u8 = 0x2A;
const ILI9341_PAGE_ADDR: u8 = 0x2B;
const ILI9341_GRAM: u8 = 0x2C;
const ILI9341_MAC: u8 = 0x36;
const ILI9341_PIXEL_FORMAT: u8 = 0x3A;
const ILI9341_WDB: u8 = 0x51;
const ILI9341_WCD: u8 = 0x53;
const ILI9341_RGB_INTERFACE: u8 = 0xB0;
const ILI9341_FRC: u8 = 0xB1;
const ILI9341_BPC: u8 = 0xB5;
const ILI9341_DFC: u8 = 0xB6;
const ILI9341_POWER1: u8 = 0xC0;
const ILI9341_POWER2: u8 = 0xC1;
const ILI9341_VCOM1: u8 = 0xC5;
const ILI9341_VCOM2: u8 = 0xC7;
const ILI9341_POWERA: u8 = 0xCB;
const ILI9341_POWERB: u8 = 0xCF;
const ILI9341_PGAMMA: u8 = 0xE0;
const ILI9341_NGAMMA: u8 = 0xE1;
const ILI9341_DTCA: u8 = 0xE8;
const ILI9341_DTCB: u8 = 0xEA;
const ILI9341_POWER_SEQ: u8 = 0xED;
const ILI9341_3GAMMA_EN: u8 = 0xF2;
const ILI9341_INTERFACE: u8 = 0xF6;
const ILI9341_PRC: u8 = 0xF7;

fn main() -> ! {
    let mut stdout = hio::hstdout().unwrap();
    let pa = arm::Peripherals::take().unwrap();
    let p = stm::Peripherals::take().unwrap();
    writeln!(stdout, "got peripherals").unwrap();

    let mut rcc = p.RCC.constrain();
    let mut flash = p.FLASH.constrain();
    let mut gpioa = p.GPIOA.split(&mut rcc.ahb1);
    let mut gpiob = p.GPIOB.split(&mut rcc.ahb1);
    let mut gpioc = p.GPIOC.split(&mut rcc.ahb1);
    let mut gpiod = p.GPIOD.split(&mut rcc.ahb1);
    let mut gpioe = p.GPIOE.split(&mut rcc.ahb1);
    let mut gpiof = p.GPIOF.split(&mut rcc.ahb1);
    let mut gpiog = p.GPIOG.split(&mut rcc.ahb1);
    let clocks = rcc.cfgr.freeze(&mut flash.acr);

    let mut time = Delay::new(pa.SYST, clocks);

    let sclk1 = gpioa.pa5.into_af5(&mut gpioa.moder, &mut gpioa.afrl);
    let miso1 = gpioa.pa6.into_af5(&mut gpioa.moder, &mut gpioa.afrl);
    let mosi1 = gpioa.pa7.into_af5(&mut gpioa.moder, &mut gpioa.afrl);

    let mut cs = gpioc.pc2.into_push_pull_output(&mut gpioc.moder, &mut gpioc.otyper);
    let mut rst = gpiod.pd12.into_push_pull_output(&mut gpiod.moder, &mut gpiod.otyper);
    let mut ds = gpiod.pd13.into_push_pull_output(&mut gpiod.moder, &mut gpiod.otyper);
    let sclk = gpiof.pf7.into_af5(&mut gpiof.moder, &mut gpiof.afrl);
    let miso = gpiof.pf8.into_af5(&mut gpiof.moder, &mut gpiof.afrl);
    let mosi = gpiof.pf9.into_af5(&mut gpiof.moder, &mut gpiof.afrh);

    let mut led1 = gpiog.pg13.into_push_pull_output(&mut gpiog.moder, &mut gpiog.otyper);
    let mut led2 = gpiog.pg14.into_push_pull_output(&mut gpiog.moder, &mut gpiog.otyper);
    led1.set_low();
    led2.set_high();
    cs.set_high();
    rst.set_low();
    time.delay_ms(10u16);
    rst.set_high();
    time.delay_ms(120u16);

    let mut display_spi = hal::spi::Spi::spi5(p.SPI5, (sclk, miso, mosi),
                                              hal_base::spi::Mode {
                                                  polarity: hal_base::spi::Polarity::IdleLow,
                                                  phase: hal_base::spi::Phase::CaptureOnFirstTransition
                                              },
                                              MegaHertz(1), clocks, &mut rcc.apb2);

    let mut display_spix = hal::spi::Spi::spi1(p.SPI1, (sclk1, miso1, mosi1),
                                              hal_base::spi::Mode {
                                                  polarity: hal_base::spi::Polarity::IdleLow,
                                                  phase: hal_base::spi::Phase::CaptureOnFirstTransition
                                              },
                                              KiloHertz(500), clocks, &mut rcc.apb2);

    macro_rules! scmd {
        ($cmd:expr) => {
            ds.set_low();
            cs.set_low();
            scmd!(@send $cmd);
            cs.set_high();
        };
        ($cmd:expr, $($data:tt)+) => {
            ds.set_low();
            cs.set_low();
            scmd!(@send $cmd);
            ds.set_high();
            scmd!(@send $($data)+);
            ds.set_low();
            cs.set_high();
        };
        (@send $($byte:expr),+) => {
            $( block!(display_spi.send($byte)).unwrap();
               time.delay_us(10u16);
            )+
        };
    }

    writeln!(stdout, "reset ...").unwrap();
    scmd!(ILI9341_RESET); // RESET
    time.delay_ms(120u16);

    scmd!(ILI9341_POWERA, 0x39, 0x2C, 0x00, 0x34, 0x02);
    scmd!(ILI9341_POWERB, 0x00, 0xC1, 0x30);
    scmd!(ILI9341_DTCA, 0x85, 0x00, 0x78);
    scmd!(ILI9341_DTCB, 0x00, 0x00);
    scmd!(ILI9341_POWER_SEQ, 0x64, 0x03, 0x12, 0x81);
    scmd!(ILI9341_PRC, 0x20);
    scmd!(ILI9341_POWER1, 0x23);
    scmd!(ILI9341_POWER2, 0x10);
    scmd!(ILI9341_VCOM1, 0x3E, 0x28);
    scmd!(ILI9341_VCOM2, 0x86);
    scmd!(ILI9341_MAC, 0x28);
    scmd!(ILI9341_PIXEL_FORMAT, 0x55);
    scmd!(ILI9341_FRC, 0x00, 0x18);
    scmd!(ILI9341_DFC, 0x08, 0x82, 0x27);
    scmd!(ILI9341_3GAMMA_EN, 0x00);
    scmd!(ILI9341_COLUMN_ADDR, 0x00, 0x00, 0x00, 0xEF);
    scmd!(ILI9341_PAGE_ADDR, 0x00, 0x00, 0x01, 0x3F);
    scmd!(ILI9341_GAMMA, 0x01);
    scmd!(ILI9341_PGAMMA, 0x0F, 0x31, 0x2B, 0x0C, 0x0E, 0x08, 0x4E, 0xF1, 0x37, 0x07, 0x10, 0x03, 0x0E, 0x09, 0x00);
    scmd!(ILI9341_NGAMMA, 0x00, 0x0E, 0x14, 0x03, 0x11, 0x07, 0x31, 0xC1, 0x48, 0x08, 0x0F, 0x0C, 0x31, 0x36, 0x0F);

    // writeln!(stdout, "sleep ...").unwrap();
    scmd!(ILI9341_SLEEP_OUT);
    time.delay_ms(120u16);
    // writeln!(stdout, "on ...").unwrap();
    scmd!(ILI9341_DISPLAY_ON);

    // writeln!(stdout, "write something ...").unwrap();
    for i in 0..320 {
        for j in 0..240 {
            scmd!(ILI9341_COLUMN_ADDR, (i >> 8) as u8, i as u8, (i >> 8) as u8, i as u8);
            scmd!(ILI9341_PAGE_ADDR, 0, j, 0, j);
            scmd!(ILI9341_GRAM, 0x0, j);
        }
    }
    scmd!(ILI9341_DISPLAY_ON);

    led1.set_high();
    led2.set_low();
    panic!("end");
    loop {
        led1.set_high();
        led2.set_low();
        time.delay_ms(500u16);
        led2.set_high();
        led1.set_low();
        time.delay_ms(500u16);
    }
}


exception!(HardFault, hard_fault);

fn hard_fault(ef: &ExceptionFrame) -> ! {
    panic!("HardFault at {:#?}", ef);
}

exception!(*, default_handler);

fn default_handler(irqn: i16) {
    panic!("Unhandled exception (IRQn = {})", irqn);
}
