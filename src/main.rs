#![no_main]
#![no_std]
#![feature(nll)]

#[macro_use]
extern crate cortex_m_rt as rt;
extern crate cortex_m as arm;
extern crate cortex_m_semihosting as sh;
extern crate panic_semihosting;
// #[macro_use]
extern crate nb;
extern crate btoi;
extern crate arraydeque;
extern crate bresenham;
extern crate embedded_hal as hal_base;
#[macro_use]
extern crate stm32f429 as stm;
extern crate stm32f429_hal as hal;

use rt::ExceptionFrame;
use arraydeque::ArrayDeque;
use hal::time::*;
use hal::timer::{Timer, Event};
use hal::rcc::RccExt;
use hal::gpio::GpioExt;
use hal::flash::FlashExt;
use hal_base::digital::OutputPin;
use core::sync::atomic::{AtomicBool, Ordering, ATOMIC_BOOL_INIT};

// use sh::hio;
// use core::fmt::Write;

#[macro_use]
mod util;
mod icon;
mod i2ceeprom;
mod spiflash;
mod interface;
mod framebuf;
mod console;

use console::Console;
use framebuf::FrameBuffer;
use interface::Action;

/// Width and height of visible screen.
const WIDTH: u16 = 480;
const HEIGHT: u16 = 128;

/// Size of a character in the console output.
const CHARW: u16 = framebuf::CONSOLEFONT.size().0;
const CHARH: u16 = framebuf::CONSOLEFONT.size().1;

/// Horizontal display timing.
const H_SYNCPULSE:  u16 = 11;
const H_BACKPORCH:  u16 = 5;
const H_ACTIVE:     u16 = WIDTH;
const H_FRONTPORCH: u16 = 28;

/// Vertical display timing.
const V_SYNCPULSE:  u16 = 2;
const V_BACKPORCH:  u16 = 3;
const V_ACTIVE:     u16 = 272;  // different from HEIGHT!
const V_FRONTPORCH: u16 = 8;

/// Upper-left corner of screen for layer windows.
const H_WIN_START:  u16 = H_SYNCPULSE + H_BACKPORCH - 1;
const V_WIN_START:  u16 = V_SYNCPULSE + V_BACKPORCH - 1;

// Graphics framebuffer
const FB_GRAPHICS_SIZE: usize = (WIDTH as usize) * (HEIGHT as usize);

#[link_section = ".sram1bss"]
static mut FB_GRAPHICS: [u8; FB_GRAPHICS_SIZE] = [0; FB_GRAPHICS_SIZE];

// Console framebuffer
// Size includes one extra row for scrolling via DMA
const FB_CONSOLE_SIZE: usize = (WIDTH as usize) * ((HEIGHT + CHARH) as usize);
#[link_section = ".sram3bss"]
static mut FB_CONSOLE: [u8; FB_CONSOLE_SIZE] = [0; FB_CONSOLE_SIZE];

// Cursor framebuffer: just the cursor itself
const CURSOR_COLOR: u8 = 127;
static CURSORBUF: [u8; CHARW as usize] = [CURSOR_COLOR; CHARW as usize];
static CURSOR_ENABLED: AtomicBool = ATOMIC_BOOL_INIT;

// TX receive buffer
static mut RXBUF: Option<ArrayDeque<[u8; 1024]>> = None;

fn fifo() -> &'static mut ArrayDeque<[u8; 1024]> {
    unsafe { RXBUF.get_or_insert_with(ArrayDeque::new) }
}

#[entry]
fn main() -> ! {
    inner_main()
}

fn inner_main() -> ! {
    // let mut stdout = hio::hstdout().unwrap();
    let pcore = arm::Peripherals::take().unwrap();
    let peri = stm::Peripherals::take().unwrap();

    // configure clock
    let mut rcc = peri.RCC.constrain();
    rcc.cfgr = rcc.cfgr.sysclk(MegaHertz(168))
        .hclk(MegaHertz(168))
        .pclk1(MegaHertz(42))
        .pclk2(MegaHertz(84));

    // activate flash caches
    write!(FLASH.acr: dcen = true, icen = true, prften = true);
    let mut flash = peri.FLASH.constrain();
    let clocks = rcc.cfgr.freeze(&mut flash.acr);
    // let mut delay = Delay::new(pcore.SYST, clocks);

    // set up pins
    let mut gpioa = peri.GPIOA.split(&mut rcc.ahb1);
    let mut gpiob = peri.GPIOB.split(&mut rcc.ahb1);
    let mut gpioc = peri.GPIOC.split(&mut rcc.ahb1);
    let mut gpiod = peri.GPIOD.split(&mut rcc.ahb1);
    let mut gpioe = peri.GPIOE.split(&mut rcc.ahb1);

    // LCD enable: set it low first to avoid LCD bleed while setting up timings
    let mut disp_on = gpioa.pa8.into_push_pull_output(&mut gpioa.moder, &mut gpioa.otyper);
    disp_on.set_low();

    // LCD backlight enable
    let mut backlight = gpiod.pd12.into_push_pull_output(&mut gpiod.moder, &mut gpiod.otyper);
    backlight.set_high();

    // Pin connected to Boot0
    let mut bootpin = gpiob.pb7.into_push_pull_output(&mut gpiob.moder, &mut gpiob.otyper);
    bootpin.set_low();

    // set up blinking timer
    let mut blink_timer = Timer::tim3(peri.TIM3, Hertz(4), clocks, &mut rcc.apb1);

    // External Flash memory via SPI
    /*
    let cs = gpiob.pb12.into_push_pull_output(&mut gpiob.moder, &mut gpiob.otyper);
    let sclk = gpiob.pb13.into_af5(&mut gpiob.moder, &mut gpiob.afrh);
    let miso = gpiob.pb14.into_af5(&mut gpiob.moder, &mut gpiob.afrh);
    let mosi = gpiob.pb15.into_af5(&mut gpiob.moder, &mut gpiob.afrh);
    let spi2 = hal::spi::Spi::spi2(peri.SPI2, (sclk, miso, mosi),
        hal_base::spi::Mode { polarity: hal_base::spi::Polarity::IdleLow,
                              phase: hal_base::spi::Phase::CaptureOnFirstTransition },
        MegaHertz(40), clocks, &mut rcc.apb1);
    let mut spi_flash = spiflash::SPIFlash::new(spi2, cs);
    */

    // Console UART (USART #1)
    let utx = gpioa.pa9 .into_af7(&mut gpioa.moder, &mut gpioa.afrh);
    let urx = gpioa.pa10.into_af7(&mut gpioa.moder, &mut gpioa.afrh);
    //let rts = gpiod.pd12.into_af7(&mut gpiod.moder, &mut gpiod.afrh);
    let mut console_uart = hal::serial::Serial::usart1(peri.USART1, (utx, urx),
        hal::time::Bps(115200), clocks, &mut rcc.apb2);
    //console_uart.set_rts(rts);
    console_uart.listen(hal::serial::Event::Rxne);
    let (console_tx, _) = console_uart.split();

    // I2C EEPROM
    let i2c_scl = gpioc.pc4.into_open_drain_output(&mut gpioc.moder, &mut gpioc.otyper);
    let i2c_sda = gpioc.pc5.into_open_drain_output(&mut gpioc.moder, &mut gpioc.otyper);
    let mut eeprom = i2ceeprom::I2CEEprom::new(i2c_scl, i2c_sda);

    // LCD pins
    gpioa.pa3 .into_lcd(&mut gpioa.moder, &mut gpioa.ospeedr, &mut gpioa.afrl, 0xE);
    gpioa.pa4 .into_lcd(&mut gpioa.moder, &mut gpioa.ospeedr, &mut gpioa.afrl, 0xE);
    gpioa.pa6 .into_lcd(&mut gpioa.moder, &mut gpioa.ospeedr, &mut gpioa.afrl, 0xE);
    gpioa.pa11.into_lcd(&mut gpioa.moder, &mut gpioa.ospeedr, &mut gpioa.afrh, 0xE);
    gpioa.pa12.into_lcd(&mut gpioa.moder, &mut gpioa.ospeedr, &mut gpioa.afrh, 0xE);
    gpiob.pb0 .into_lcd(&mut gpiob.moder, &mut gpiob.ospeedr, &mut gpiob.afrl, 0x9);
    gpiob.pb1 .into_lcd(&mut gpiob.moder, &mut gpiob.ospeedr, &mut gpiob.afrl, 0x9);
    gpiob.pb8 .into_lcd(&mut gpiob.moder, &mut gpiob.ospeedr, &mut gpiob.afrh, 0xE);
    gpiob.pb9 .into_lcd(&mut gpiob.moder, &mut gpiob.ospeedr, &mut gpiob.afrh, 0xE);
    gpiob.pb10.into_lcd(&mut gpiob.moder, &mut gpiob.ospeedr, &mut gpiob.afrh, 0xE);
    gpiob.pb11.into_lcd(&mut gpiob.moder, &mut gpiob.ospeedr, &mut gpiob.afrh, 0xE);
    gpioc.pc6 .into_lcd(&mut gpioc.moder, &mut gpioc.ospeedr, &mut gpioc.afrl, 0xE);
    gpioc.pc7 .into_lcd(&mut gpioc.moder, &mut gpioc.ospeedr, &mut gpioc.afrl, 0xE);
    gpioc.pc10.into_lcd(&mut gpioc.moder, &mut gpioc.ospeedr, &mut gpioc.afrh, 0xE);
    gpiod.pd3 .into_lcd(&mut gpiod.moder, &mut gpiod.ospeedr, &mut gpiod.afrl, 0xE);
    gpiod.pd6 .into_lcd(&mut gpiod.moder, &mut gpiod.ospeedr, &mut gpiod.afrl, 0xE);
    gpiod.pd10.into_lcd(&mut gpiod.moder, &mut gpiod.ospeedr, &mut gpiod.afrh, 0xE);
    gpioe.pe11.into_lcd(&mut gpioe.moder, &mut gpioe.ospeedr, &mut gpioe.afrh, 0xE);
    gpioe.pe12.into_lcd(&mut gpioe.moder, &mut gpioe.ospeedr, &mut gpioe.afrh, 0xE);
    gpioe.pe13.into_lcd(&mut gpioe.moder, &mut gpioe.ospeedr, &mut gpioe.afrh, 0xE);
    gpioe.pe14.into_lcd(&mut gpioe.moder, &mut gpioe.ospeedr, &mut gpioe.afrh, 0xE);
    gpioe.pe15.into_lcd(&mut gpioe.moder, &mut gpioe.ospeedr, &mut gpioe.afrh, 0xE);

    // enable clocks
    modif!(RCC.apb2enr: ltdcen = true);
    modif!(RCC.ahb1enr: dma2den = true);
    // enable PLLSAI
    // PLLSAI_VCO Input = HSE_VALUE/PLL_M = 1 Mhz
    // PLLSAI_VCO Output = PLLSAI_VCO Input * PLLSAI_N = 216 Mhz (f=100..432 MHz)
    // PLLLCDCLK = PLLSAI_VCO Output/PLLSAI_R = 216/3 = 72 Mhz  (r=2..7)
    // LTDC clock frequency = PLLLCDCLK / RCC_PLLSAIDivR = 72/8 = 9 Mhz (/2 /4 /8 /16)
    write!(RCC.pllsaicfgr: pllsain = 216, pllsaiq = 7, pllsair = 3);
    write!(RCC.dckcfgr: pllsaidivr = 0b10);  // divide by 8
    // enable PLLSAI and wait for it
    modif!(RCC.cr: pllsaion = true);
    wait_for!(RCC.cr: pllsairdy);

    // Basic ChromArt configuration
    write!(DMA2D.fgpfccr: cm = 0b0101);  // L8 in/out

    // Configure LCD timings
    write!(LTDC.sscr: hsw = H_SYNCPULSE - 1, vsh = V_SYNCPULSE - 1); // -1 required by STM
    write!(LTDC.bpcr: ahbp = H_WIN_START, avbp = V_WIN_START);
    write!(LTDC.awcr: aav = H_WIN_START + H_ACTIVE, aah = V_WIN_START + V_ACTIVE);
    write!(LTDC.twcr: totalw = H_WIN_START + H_ACTIVE + H_FRONTPORCH,
           totalh = V_WIN_START + V_ACTIVE + V_FRONTPORCH);

    // Configure layer 1 (main framebuffer)

    // Horizontal and vertical window (coordinates include porches)
    write!(LTDC.l1whpcr: whstpos = H_WIN_START + 1, whsppos = H_WIN_START + WIDTH);
    write!(LTDC.l1wvpcr: wvstpos = V_WIN_START + 1, wvsppos = V_WIN_START + HEIGHT);
    // Pixel format
    write!(LTDC.l1pfcr: pf = 0b101);  // 8-bit (CLUT enabled below)
    // Constant alpha value
    write!(LTDC.l1cacr: consta = 0xFF);
    // Default color values
    write!(LTDC.l1dccr: dcalpha = 0, dcred = 0, dcgreen = 0, dcblue = 0);
    // Blending factors
    write!(LTDC.l1bfcr: bf1 = 4, bf2 = 5);  // Constant alpha
    // Color frame buffer start address
    write!(LTDC.l1cfbar: cfbadd = FB_CONSOLE.as_ptr() as u32);
    // Color frame buffer line length (active*bpp + 3), and pitch
    write!(LTDC.l1cfblr: cfbll = WIDTH + 3, cfbp = WIDTH);
    // Frame buffer number of lines
    write!(LTDC.l1cfblnr: cfblnbr = HEIGHT);
    // Set up 256-color LUT
    for (i, (r, g, b)) in Console::get_lut_colors().enumerate() {
        write!(LTDC.l1clutwr: clutadd = i as u8, red = r, green = g, blue = b);
    }

    // Configure layer 2 (cursor)

    // initial position: top left character
    write!(LTDC.l2whpcr: whstpos = H_WIN_START + 1, whsppos = H_WIN_START + CHARW);
    write!(LTDC.l2wvpcr: wvstpos = V_WIN_START + CHARH, wvsppos = V_WIN_START + CHARH);
    write!(LTDC.l2pfcr: pf = 0b101);  // L-8 without CLUT
    write!(LTDC.l2cacr: consta = 0xFF);
    write!(LTDC.l2dccr: dcalpha = 0, dcred = 0, dcgreen = 0, dcblue = 0);
    write!(LTDC.l2bfcr: bf1 = 6, bf2 = 7);  // Constant alpha * Pixel alpha
    write!(LTDC.l2cfbar: cfbadd = CURSORBUF.as_ptr() as u32);
    write!(LTDC.l2cfblr: cfbll = CHARW + 3, cfbp = CHARW);
    write!(LTDC.l2cfblnr: cfblnbr = 1);  // Cursor is one line of 6 pixels

    // Enable layer1, disable layer2 initially
    modif!(LTDC.l1cr: cluten = true, len = true);
    modif!(LTDC.l2cr: len = false);

    // Reload config again
    write!(LTDC.srcr: imr = true);  // Immediate reload

    // Dither on, display on
    modif!(LTDC.gcr: den = true, ltdcen = true);

    // Reload config to show display
    write!(LTDC.srcr: imr = true);

    // enable external display
    disp_on.set_high();

    // enable interrupts
    let mut nvic = pcore.NVIC;
    nvic.enable(stm::Interrupt::TIM3);
    blink_timer.listen(Event::TimeOut);

    let console = console::Console::new(
        FrameBuffer::new(unsafe { &mut FB_CONSOLE }, WIDTH, HEIGHT, true),
        console_tx
    );
    let mut disp = interface::DisplayState::new(
        FrameBuffer::new(unsafe { &mut FB_GRAPHICS }, WIDTH, HEIGHT, false),
        console
    );

    disp.console().activate();

    // load pre-programmed startup sequence from EEPROM
    let mut startup_len = [0, 0];
    let mut startup_buf = [0; 256];
    if eeprom.read_at_addr(0, &mut startup_len).is_ok() {
        let startup_len = (startup_len[0] as usize) | ((startup_len[1] as usize) << 8);
        // this excludes the unprogrammed case of 0xffff
        if startup_len > 0 && startup_len <= startup_buf.len() {
            if eeprom.read_at_addr(64, &mut startup_buf).is_ok() {
                for &byte in startup_buf[..startup_len].iter() {
                    let _ = fifo().push_back(byte);
                }
            }
        }
    }

    nvic.enable(stm::Interrupt::USART1);

    // main loop: process input
    loop {
        if let Some(ch) = arm::interrupt::free(|_| fifo().pop_front()) {
            match disp.process_byte(ch) {
                Action::None => (),
                Action::Reset => reset(pcore.SCB),
                Action::Bootloader => reset_to_bootloader(pcore.SCB, bootpin),
                Action::WriteEeprom(len_addr, data_addr, data) => {
                    assert!(data_addr % 64 == 0);
                    if eeprom.write_at_addr(len_addr, &[data.len() as u8, 0]).is_ok() {
                        for (addr, chunk) in (data_addr..).step_by(64).zip(data.chunks(64)) {
                            let _ = eeprom.write_at_addr(addr, chunk);
                        }
                    }
                }
            }
        }
    }
}

interrupt!(TIM3, blink, state: bool = false);

pub fn enable_cursor(en: bool) {
    CURSOR_ENABLED.store(en, Ordering::Relaxed);
}

fn blink(visible: &mut bool) {
    // toggle layer2 on next vsync
    *visible = !*visible;
    modif!(LTDC.l2cr: len = bit(CURSOR_ENABLED.load(Ordering::Relaxed) && *visible));
    write!(LTDC.srcr: vbr = true);
    // reset timer
    modif!(TIM3.sr: uif = false);
    modif!(TIM3.cr1: cen = true);
}

interrupt!(USART1, receive);

fn receive() {
    let data = read!(USART1.dr: dr) as u8;
    let _ = fifo().push_back(data);
}

#[exception]
fn HardFault(ef: &ExceptionFrame) -> ! {
    panic!("HardFault at {:#?}", ef);
}

#[exception]
fn DefaultHandler(irqn: i16) {
    panic!("Unhandled exception (IRQn = {})", irqn);
}

const SCB_AIRCR_RESET: u32 = 0x05FA_0004;

pub fn reset(scb: stm::SCB) -> ! {
    unsafe {
        arm::interrupt::disable();
        arm::asm::dsb();
        // do a soft-reset of the cpu
        scb.aircr.write(SCB_AIRCR_RESET);
        arm::asm::dsb();
        unreachable!()
    }
}

pub fn reset_to_bootloader<O: OutputPin>(scb: stm::SCB, mut pin: O) -> ! {
    unsafe {
        arm::interrupt::disable();
        // set boot0 high (keeps high through reset via RC circuit)
        pin.set_high();
        arm::asm::delay(10000);
        arm::asm::dsb();
        // do a soft-reset of the cpu
        scb.aircr.write(SCB_AIRCR_RESET);
        arm::asm::dsb();
        unreachable!()
    }
}
