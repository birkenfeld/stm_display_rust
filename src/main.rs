#![no_main]
#![no_std]
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
use stm::RCC;

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

static mut FRAMEBUF: [u16; 240*320] = [0; 240*320];

fn main() -> ! {

    let mut stdout = hio::hstdout().unwrap();
    let pa = arm::Peripherals::take().unwrap();
    let p = stm::Peripherals::take().unwrap();
    writeln!(stdout, "start...").unwrap();

    let mut rcc = p.RCC.constrain();
    let mut flash = p.FLASH.constrain();
    let clocks = rcc.cfgr.freeze(&mut flash.acr);

unsafe {
    for i in 0..1 {
        let base = (0x4002_0000 + i*0x400) as *const u32;
        writeln!(stdout, "GPIO {}: {:#010x} {:#010x} {:#010x} {:#010x}", i,
                 ptr::read_volatile(base), ptr::read_volatile(base.offset(1)),
                 ptr::read_volatile(base.offset(2)), ptr::read_volatile(base.offset(3))).unwrap();
    }
}

    // configure LCD pins
    p.GPIOA.moder.modify(|_, w| unsafe {
        w.moder3().bits(0b10).moder4().bits(0b10).moder6().bits(0b10).moder11().bits(0b10).moder12().bits(0b10)
    });
    p.GPIOA.ospeedr.modify(|_, w| unsafe {
        w.ospeedr3().bits(0b10).ospeedr4().bits(0b10).ospeedr6().bits(0b10).ospeedr11().bits(0b10).ospeedr12().bits(0b10)
    });
    p.GPIOA.afrl.modify(|_, w| unsafe {
        w.afrl3().bits(0xE).afrl4().bits(0xE).afrl6().bits(0xE)
    });
    p.GPIOA.afrh.modify(|_, w| unsafe {
        w.afrh11().bits(0xE).afrh12().bits(0xE)
    });

    p.GPIOB.moder.modify(|_, w| unsafe {
        w.moder0().bits(0b10).moder1().bits(0b10).moder8().bits(0b10).moder9().bits(0b10).moder10().bits(0b10).moder11().bits(0b10)
    });
    p.GPIOB.ospeedr.modify(|_, w| unsafe {
        w.ospeedr0().bits(0b10).ospeedr1().bits(0b10).ospeedr8().bits(0b10).ospeedr9().bits(0b10).ospeedr10().bits(0b10).ospeedr11().bits(0b10)
    });
    p.GPIOB.afrl.modify(|_, w| unsafe {
        w.afrl0().bits(0x9).afrl1().bits(0x9)
    });
    p.GPIOB.afrh.modify(|_, w| unsafe {
        w.afrh8().bits(0xE).afrh9().bits(0xE).afrh10().bits(0xE).afrh11().bits(0xE)
    });

    p.GPIOC.moder.modify(|_, w| unsafe {
        w.moder6().bits(0b10).moder7().bits(0b10).moder10().bits(0b10)
    });
    p.GPIOC.ospeedr.modify(|_, w| unsafe {
        w.ospeedr6().bits(0b10).ospeedr7().bits(0b10).ospeedr10().bits(0b10)
    });
    p.GPIOC.afrl.modify(|_, w| unsafe {
        w.afrl6().bits(0xE).afrl7().bits(0xE)
    });
    p.GPIOC.afrh.modify(|_, w| unsafe {
        w.afrh10().bits(0xE)
    });

    p.GPIOD.moder.modify(|_, w| unsafe {
        w.moder3().bits(0b10).moder6().bits(0b10)
    });
    p.GPIOD.ospeedr.modify(|_, w| unsafe {
        w.ospeedr3().bits(0b10).ospeedr6().bits(0b10)
    });
    p.GPIOD.afrl.modify(|_, w| unsafe {
        w.afrl3().bits(0xE).afrl6().bits(0xE)
    });

    p.GPIOF.moder.modify(|_, w| unsafe {
        w.moder10().bits(0b10)
    });
    p.GPIOF.ospeedr.modify(|_, w| unsafe {
        w.ospeedr10().bits(0b10)
    });
    p.GPIOF.afrh.modify(|_, w| unsafe {
        w.afrh10().bits(0xE)
    });

    p.GPIOG.moder.modify(|_, w| unsafe {
        w.moder6().bits(0b10).moder7().bits(0b10).moder10().bits(0b10).moder11().bits(0b10).moder12().bits(0b10)
    });
    p.GPIOG.ospeedr.modify(|_, w| unsafe {
        w.ospeedr6().bits(0b10).ospeedr7().bits(0b10).ospeedr10().bits(0b10).ospeedr11().bits(0b10).ospeedr12().bits(0b10)
    });
    p.GPIOG.afrl.modify(|_, w| unsafe {
        w.afrl6().bits(0xE).afrl7().bits(0xE)
    });
    p.GPIOG.afrh.modify(|_, w| unsafe {
        w.afrh11().bits(0xE).afrh10().bits(0x9).afrh12().bits(0x9)
    });

    use core::ptr;

unsafe {
    for i in 0..1 {
        let base = (0x4002_0000 + i*0x400) as *const u32;
        writeln!(stdout, "GPIO {}: {:#010x} {:#010x} {:#010x} {:#010x}", i,
                 ptr::read_volatile(base), ptr::read_volatile(base.offset(1)),
                 ptr::read_volatile(base.offset(2)), ptr::read_volatile(base.offset(3))).unwrap();
    }
}

    // enable LTDC clock
    let rcc_raw = unsafe { &*RCC::ptr() };
    rcc_raw.apb2enr.modify(|_, w| w.ltdcen().bit(true));
    // enable DMA2D clock
    rcc_raw.ahb1enr.modify(|_, w| w.dma2den().bit(true));
    // enable PLLSAI
    	/* Configure PLLSAI prescalers for LCD */
	/* Enable Pixel Clock */
	/* PLLSAI_VCO Input = HSE_VALUE/PLL_M = 1 Mhz */
	/* PLLSAI_VCO Output = PLLSAI_VCO Input * PLLSAI_N = 192 Mhz */
	/* PLLLCDCLK = PLLSAI_VCO Output/PLLSAI_R = 192/4 = 96 Mhz */
	/* LTDC clock frequency = PLLLCDCLK / RCC_PLLSAIDivR = 96/4 = 24 Mhz */
    rcc_raw.pllsaicfgr.write(|w| unsafe { w.pllsain().bits(192)
                                       .pllsaiq().bits(7)
                                       .pllsair().bits(4) });
    rcc_raw.dckcfgr.modify(|_, w| unsafe { w.pllsaidivr().bits(0b01) }); // div4
    // enable PLLSAI and wait for it
    rcc_raw.cr.modify(|_, w| w.pllsaion().bit(true));
    while !rcc_raw.cr.read().pllsairdy().bit() {}

    // writeln!(stdout, "RCC:").unwrap();
    // for n in 0..

    // Vsync, Hsync
    p.LTDC.sscr.write(|w| unsafe { w.vsh().bits(1).hsw().bits(9) });
    // Back porch
    p.LTDC.bpcr.write(|w| unsafe { w.avbp().bits(3).ahbp().bits(29) });
    // Active width
    p.LTDC.awcr.write(|w| unsafe { w.aah().bits(323).aaw().bits(269) });
    // Total width
    p.LTDC.twcr.write(|w| unsafe { w.totalh().bits(327).totalw().bits(279) });
    // Global control reg -- all signals active low, clock is as input
    p.LTDC.gcr.modify(|_, w| w.hspol().bit(false)
                              .vspol().bit(false)
                              .depol().bit(false)
                              .pcpol().bit(false));
    // Background color
    p.LTDC.bccr.write(|w| unsafe { w.bc().bits(0x0000FF) });

    // Configure layer1

    // Horizontal start/stop
    p.LTDC.l1whpcr.write(|w| unsafe { w.whstpos().bits(30).whsppos().bits(269) });
    // Vertical start/stop
    p.LTDC.l1wvpcr.write(|w| unsafe { w.wvstpos().bits(4).wvsppos().bits(323) });
    // Pixel format
    p.LTDC.l1pfcr.write(|w| unsafe { w.pf().bits(0b010) }); // RGB-565
    // Constant alpha value
    p.LTDC.l1cacr.write(|w| unsafe { w.consta().bits(0) });
    // Default color values
    p.LTDC.l1dccr.write(|w| unsafe { w.dcalpha().bits(0).dcred().bits(0).dcgreen().bits(0).dcblue().bits(0) });
    // Blending factors
    p.LTDC.l1bfcr.write(|w| unsafe { w.bf1().bits(4).bf2().bits(4) }); // Constant alpha
    // Color frame buffer start address
    p.LTDC.l1cfbar.write(|w| unsafe { w.cfbadd().bits(FRAMEBUF.as_ptr() as u32) }); // XXX
    // Color frame buffer line length (active*bpp + 3), and pitch
    p.LTDC.l1cfblr.write(|w| unsafe { w.cfbll().bits(240*2 + 3).cfbp().bits(240*2) });
    // Frame buffer number of lines
    p.LTDC.l1cfblnr.write(|w| unsafe { w.cfblnbr().bits(320) });

    // Configure layer2

    // Horizontal start/stop
    p.LTDC.l2whpcr.write(|w| unsafe { w.whstpos().bits(30).whsppos().bits(269) });
    // Vertical start/stop
    p.LTDC.l2wvpcr.write(|w| unsafe { w.wvstpos().bits(4).wvsppos().bits(323) });
    // Pixel format
    p.LTDC.l2pfcr.write(|w| unsafe { w.pf().bits(0b010) }); // RGB-565
    // Constant alpha value
    p.LTDC.l2cacr.write(|w| unsafe { w.consta().bits(0) });
    // Default color values
    p.LTDC.l2dccr.write(|w| unsafe { w.dcalpha().bits(0).dcred().bits(0).dcgreen().bits(0).dcblue().bits(0) });
    // Blending factors
    p.LTDC.l2bfcr.write(|w| unsafe { w.bf1().bits(6).bf2().bits(6) }); // Constant alpha * Pixel alpha
    // Color frame buffer start address
    p.LTDC.l2cfbar.write(|w| unsafe { w.cfbadd().bits(FRAMEBUF.as_ptr() as u32) }); // XXX
    // Color frame buffer line length (active*bpp + 3), and pitch
    p.LTDC.l2cfblr.write(|w| unsafe { w.cfbll().bits(240*2 + 3).cfbp().bits(240*2) });
    // Frame buffer number of lines
    p.LTDC.l2cfblnr.write(|w| unsafe { w.cfblnbr().bits(320) });

    // Reload config
    p.LTDC.srcr.write(|w| w.imr().bit(true));

    // Enable layer1, disable layer2
    p.LTDC.l1cr.modify(|_, w| w.len().bit(true));
    p.LTDC.l2cr.modify(|_, w| w.len().bit(false));

    // Reload config again
    p.LTDC.srcr.write(|w| w.imr().bit(true));

    // HAL wrapped stuff
    let mut gpioc = p.GPIOC.split(&mut rcc.ahb1);
    let mut gpiod = p.GPIOD.split(&mut rcc.ahb1);
    let mut gpiof = p.GPIOF.split(&mut rcc.ahb1);
    let mut gpiog = p.GPIOG.split(&mut rcc.ahb1);

    let mut time = Delay::new(pa.SYST, clocks);

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
               time.delay_us(7u16);
            )+
        };
    }

    scmd!(ILI9341_RESET); // RESET
    time.delay_ms(120u16);

    scmd!(0xCA, 0xC3, 0x08, 0x50);
    scmd!(ILI9341_POWERB, 0x00, 0xC1, 0x30);
    scmd!(ILI9341_POWER_SEQ, 0x64, 0x03, 0x12, 0x81);
    scmd!(ILI9341_DTCA, 0x85, 0x00, 0x78);
    scmd!(ILI9341_POWERA, 0x39, 0x2C, 0x00, 0x34, 0x02);
    scmd!(ILI9341_PRC, 0x20);
    scmd!(ILI9341_DTCB, 0x00, 0x00);
    scmd!(ILI9341_FRC, 0x00, 0x1B);
    scmd!(ILI9341_DFC, 0x0A, 0xA2);
    scmd!(ILI9341_POWER1, 0x10);
    scmd!(ILI9341_POWER2, 0x10);
    scmd!(ILI9341_VCOM1, 0x45, 0x15);
    scmd!(ILI9341_VCOM2, 0x90);
    scmd!(ILI9341_MAC, 0xC8);
    scmd!(ILI9341_3GAMMA_EN, 0x00);
    scmd!(ILI9341_RGB_INTERFACE, 0xC2);
    scmd!(ILI9341_DFC, 0x0A, 0xA7, 0x27, 0x04);

    scmd!(ILI9341_COLUMN_ADDR, 0x00, 0x00, 0x00, 0xEF);
    scmd!(ILI9341_PAGE_ADDR, 0x00, 0x00, 0x01, 0x3F);
    scmd!(ILI9341_INTERFACE, 0x01, 0x00, 0x06);
    scmd!(ILI9341_GRAM);
    time.delay_ms(120u16);
    scmd!(ILI9341_GAMMA, 0x01);
    scmd!(ILI9341_PGAMMA, 0x0F, 0x29, 0x24, 0x0C, 0x0E, 0x09, 0x4E, 0x78, 0x3C, 0x09, 0x13, 0x05, 0x17, 0x11, 0x00);
    scmd!(ILI9341_NGAMMA, 0x00, 0x16, 0x1B, 0x04, 0x11, 0x07, 0x31, 0x33, 0x42, 0x05, 0x0C, 0x0A, 0x28, 0x2F, 0x0F);
    scmd!(ILI9341_SLEEP_OUT);
    time.delay_ms(120u16);
    scmd!(ILI9341_DISPLAY_ON);
    scmd!(ILI9341_GRAM);

    // Dither on, display on
    p.LTDC.gcr.modify(|_, w| w.den().bit(true).ltdcen().bit(true));

    // Reload config again
    p.LTDC.srcr.write(|w| w.imr().bit(true));

    // writeln!(stdout, "write something ...").unwrap();
    for i in 0..320 {
        for j in 0..240 {
            unsafe {
                FRAMEBUF[i*240+j] = (i*j) as u16;
            }
        }
    }

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
