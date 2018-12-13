#![no_main]
#![no_std]

use panic_semihosting;
use stm32f4::stm32f429 as stm;
use cortex_m_rt::ExceptionFrame;
use heapless::spsc::Queue;
use heapless::consts::*;
use hal::time::*;
use hal::timer::{Timer, Event};
use hal::serial::{Serial, config::Config as SerialConfig};
use hal::rcc::RccExt;
use hal::gpio::{GpioExt, Speed};
use embedded_hal::digital::OutputPin;
use core::sync::atomic::{AtomicBool, Ordering, ATOMIC_BOOL_INIT};

#[macro_use]
mod util;
mod icon;
mod i2ceeprom;
mod spiflash;
mod interface;
mod framebuf;
mod console;

use crate::console::Console;
use crate::framebuf::FrameBuffer;
use crate::interface::Action;

/// Reply to host's identify query.
const IDENT: [u8; 4] = [0x00, 0x00, 0x01, 0x00];

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

// UART receive buffer
static mut UART_RX: Queue<u8, U1024, u16> = Queue::u16();

// Touch event buffer
static mut TOUCH_EVT: Queue<u16, U16, u8> = Queue::u8();

#[cortex_m_rt::entry]
fn main() -> ! {
    // let mut stdout = hio::hstdout().unwrap();
    let pcore = arm::Peripherals::take().unwrap();
    let peri = stm::Peripherals::take().unwrap();

    // Configure clock
    let mut rcc = peri.RCC.constrain();
    rcc.cfgr = rcc.cfgr.sysclk(MegaHertz(168))
        .hclk(MegaHertz(168))
        .pclk1(MegaHertz(42))
        .pclk2(MegaHertz(84));
    let clocks = rcc.cfgr.freeze();

    // Activate flash caches
    modif!(FLASH.acr: dcen = true, icen = true, prften = true);
    // let mut delay = Delay::new(pcore.SYST, clocks);

    // Set up pins
    let gpioa = peri.GPIOA.split();
    let gpiob = peri.GPIOB.split();
    let gpioc = peri.GPIOC.split();
    let gpiod = peri.GPIOD.split();
    let gpioe = peri.GPIOE.split();

    // LCD enable: set it low first to avoid LCD bleed while setting up timings
    let mut disp_on = gpioa.pa8.into_push_pull_output();
    disp_on.set_low();

    // LCD backlight enable
    let mut backlight = gpiod.pd12.into_push_pull_output();
    backlight.set_high();

    // Output pin connected to Boot0 + capacitor
    let mut bootpin = gpiob.pb7.into_push_pull_output();
    bootpin.set_low();

    // Set up blinking timer
    let mut blink_timer = Timer::tim3(peri.TIM3, Hertz(4), clocks);

    // Set up touch detection timer
    let mut touch_timer = Timer::tim4(peri.TIM4, Hertz(100), clocks);

    // External Flash memory via SPI
    /*
    let cs = gpiob.pb12.into_push_pull_output(&mut gpiob.moder, &mut gpiob.otyper);
    let sclk = gpiob.pb13.into_af5(&mut gpiob.moder, &mut gpiob.afrh);
    let miso = gpiob.pb14.into_af5(&mut gpiob.moder, &mut gpiob.afrh);
    let mosi = gpiob.pb15.into_af5(&mut gpiob.moder, &mut gpiob.afrh);
    let spi2 = hal::spi::Spi::spi2(peri.SPI2, (sclk, miso, mosi),
        hal_base::spi::MODE_0, MegaHertz(40), clocks, &mut rcc.apb1);
    let mut spi_flash = spiflash::SPIFlash::new(spi2, cs);
    */

    // Console UART (USART #1)
    let utx = gpioa.pa9 .into_alternate_af7();
    let urx = gpioa.pa10.into_alternate_af7();
    let uart = Serial::usart1(peri.USART1, (utx, urx),
                              SerialConfig::default().baudrate(Bps(115200)),
                              clocks).unwrap();
    let (console_tx, _) = uart.split();

    // I2C EEPROM
    let i2c_scl = gpioc.pc4.into_open_drain_output();
    let i2c_sda = gpioc.pc5.into_open_drain_output();
    let mut eeprom = i2ceeprom::I2CEEprom::new(i2c_scl, i2c_sda);

    // LCD pins
    gpioa.pa3 .into_alternate_af14().set_speed(Speed::VeryHigh);
    gpioa.pa4 .into_alternate_af14().set_speed(Speed::VeryHigh);
    gpioa.pa6 .into_alternate_af14().set_speed(Speed::VeryHigh);
    gpioa.pa11.into_alternate_af14().set_speed(Speed::VeryHigh);
    gpioa.pa12.into_alternate_af14().set_speed(Speed::VeryHigh);
    gpiob.pb0 .into_alternate_af9() .set_speed(Speed::VeryHigh);
    gpiob.pb1 .into_alternate_af9() .set_speed(Speed::VeryHigh);
    gpiob.pb8 .into_alternate_af14().set_speed(Speed::VeryHigh);
    gpiob.pb9 .into_alternate_af14().set_speed(Speed::VeryHigh);
    gpiob.pb10.into_alternate_af14().set_speed(Speed::VeryHigh);
    gpiob.pb11.into_alternate_af14().set_speed(Speed::VeryHigh);
    gpioc.pc6 .into_alternate_af14().set_speed(Speed::VeryHigh);
    gpioc.pc7 .into_alternate_af14().set_speed(Speed::VeryHigh);
    gpioc.pc10.into_alternate_af14().set_speed(Speed::VeryHigh);
    gpiod.pd3 .into_alternate_af14().set_speed(Speed::VeryHigh);
    gpiod.pd6 .into_alternate_af14().set_speed(Speed::VeryHigh);
    gpiod.pd10.into_alternate_af14().set_speed(Speed::VeryHigh);
    gpioe.pe11.into_alternate_af14().set_speed(Speed::VeryHigh);
    gpioe.pe12.into_alternate_af14().set_speed(Speed::VeryHigh);
    gpioe.pe13.into_alternate_af14().set_speed(Speed::VeryHigh);
    gpioe.pe14.into_alternate_af14().set_speed(Speed::VeryHigh);
    gpioe.pe15.into_alternate_af14().set_speed(Speed::VeryHigh);

    // Pins for touch screen
    let _touch_yd = gpioc.pc0.into_pull_down_input();
    let _touch_yu = gpioc.pc1.into_floating_input();  // LATER: analog
    let mut touch_xl = gpioc.pc2.into_push_pull_output();
    touch_xl.set_low();
    let mut touch_xr = gpioc.pc3.into_push_pull_output();
    touch_xr.set_high();

    // Set yu input pin to analog mode.  Hardcoded for now!
    modif!(GPIOC.moder: moder1 = 0b11);

    // Activate and configure ADC.
    modif!(RCC.apb2enr: adc1en = true);
    // One conversion of channel 11, continuous mode.
    write!(ADC1.sqr1: l = 0);
    write!(ADC1.sqr3: sq1 = 11);
    write!(ADC1.cr2: cont = true, adon = true, swstart = true);

    // Enable clocks
    modif!(RCC.apb2enr: ltdcen = true);
    modif!(RCC.ahb1enr: dma2den = true);
    // Enable PLLSAI for LTDC
    //   PLLSAI_VCO Input = HSE_VALUE/PLL_M = 1 Mhz
    //   PLLSAI_VCO Output = PLLSAI_VCO Input * PLLSAI_N = 216 Mhz (f=100..432 MHz)
    //   PLLLCDCLK = PLLSAI_VCO Output/PLLSAI_R = 216/3 = 72 Mhz  (r=2..7)
    //   LTDC clock frequency = PLLLCDCLK / RCC_PLLSAIDivR = 72/8 = 9 Mhz (/2 /4 /8 /16)
    write!(RCC.pllsaicfgr: pllsain = 216, pllsaiq = 7, pllsair = 3);
    write!(RCC.dckcfgr: pllsaidivr = 0b10);  // divide by 8
    // Enable PLLSAI and wait for it
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

    // Initial position: top left character
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

    // Reload config (immediate)
    write!(LTDC.srcr: imr = true);

    // Dither on, display on
    modif!(LTDC.gcr: den = true, ltdcen = true);

    // Reload config to show display
    write!(LTDC.srcr: imr = true);

    // Enable display via GPIO too
    disp_on.set_high();

    // Enable interrupts
    let mut nvic = pcore.NVIC;
    nvic.enable(stm::Interrupt::TIM3);
    nvic.enable(stm::Interrupt::TIM4);
    blink_timer.listen(Event::TimeOut);
    touch_timer.listen(Event::TimeOut);

    let console = console::Console::new(
        FrameBuffer::new(unsafe { &mut FB_CONSOLE }, WIDTH, HEIGHT, true),
        console_tx
    );
    let mut disp = interface::DisplayState::new(
        FrameBuffer::new(unsafe { &mut FB_GRAPHICS }, WIDTH, HEIGHT, false),
        console
    );

    // Switch to console if nothing else programmed
    disp.console().activate();

    // Load pre-programmed startup sequence from EEPROM
    let mut startup_buf = [0; 256];
    if let Ok(code) = eeprom.read_stored_entry(0, 64, &mut startup_buf) {
        for &byte in code {
            unsafe { UART_RX.enqueue_unchecked(byte); }
        }
    }

    // Activate USART receiver
    modif!(USART1.cr1: rxneie = true);
    nvic.enable(stm::Interrupt::USART1);

    // Main loop: process input from UART
    let mut fifo = unsafe { UART_RX.split().1 };
    let mut touch = unsafe { TOUCH_EVT.split().1 };
    loop {
        if let Some(mut te) = touch.dequeue() {
            let mut c = [b'0'; 4];
            for p in c.iter_mut().rev() {
                *p += te as u8 % 10;
                te /= 10;
            }
            for p in &c {
                disp.process_byte(*p);
            }
            disp.process_byte(b' ');
        }
        if let Some(ch) = fifo.dequeue() {
            match disp.process_byte(ch) {
                Action::None => (),
                Action::Reset => reset(pcore.SCB),
                Action::Bootloader => reset_to_bootloader(pcore.SCB, bootpin),
                Action::WriteEeprom(len_addr, data_addr, data) => {
                    let _ = eeprom.write_stored_entry(len_addr, data_addr, data);
                }
            }
        }
    }
}

stm32f4::interrupt!(TIM3, blink, state: bool = false);

pub fn enable_cursor(en: bool) {
    CURSOR_ENABLED.store(en, Ordering::Relaxed);
}

fn blink(visible: &mut bool) {
    // Toggle layer2 on next vsync
    *visible = !*visible;
    modif!(LTDC.l2cr: len = bit(CURSOR_ENABLED.load(Ordering::Relaxed) && *visible));
    write!(LTDC.srcr: vbr = true);
    // Reset timer
    modif!(TIM3.sr: uif = false);
    modif!(TIM3.cr1: cen = true);
}

stm32f4::interrupt!(USART1, receive);

fn receive() {
    let data = read!(USART1.dr: dr) as u8;
    unsafe { let _ = UART_RX.split().0.enqueue(data); }
}

const THRESHOLD: u32 = 1000;
const NSAMPLES: usize = 32;

struct TouchState {
    last: bool,
    data: [u16; NSAMPLES],
    idx: usize,
}

stm32f4::interrupt!(TIM4, touch_detect, state: TouchState =
                    TouchState { last: false, data: [0; NSAMPLES], idx: 0});

fn touch_detect(state: &mut TouchState) {
    let data = read!(ADC1.dr: data);
    state.data[state.idx] = data;
    state.idx = (state.idx + 1) % NSAMPLES;
    let mean = state.data.iter().map(|&v| v as u32).sum::<u32>() / NSAMPLES as u32;
    if !state.last && mean > THRESHOLD {
        unsafe { let _ = TOUCH_EVT.split().0.enqueue(mean as u16); }
        state.last = true;
    } else if state.last && mean < THRESHOLD {
        state.last = false;
    }
}

#[cortex_m_rt::exception]
fn HardFault(ef: &ExceptionFrame) -> ! {
    panic!("HardFault at {:#?}", ef);
}

#[cortex_m_rt::exception]
fn DefaultHandler(irqn: i16) {
    panic!("Unhandled exception (IRQn = {})", irqn);
}

const SCB_AIRCR_RESET: u32 = 0x05FA_0004;

pub fn reset(scb: stm::SCB) -> ! {
    unsafe {
        arm::interrupt::disable();
        arm::asm::dsb();
        // Do a soft-reset of the cpu
        scb.aircr.write(SCB_AIRCR_RESET);
        arm::asm::dsb();
        unreachable!()
    }
}

pub fn reset_to_bootloader<O: OutputPin>(scb: stm::SCB, mut pin: O) -> ! {
    unsafe {
        arm::interrupt::disable();
        // Set Boot0 high (keeps high through reset via RC circuit)
        pin.set_high();
        arm::asm::delay(10000);
        arm::asm::dsb();
        scb.aircr.write(SCB_AIRCR_RESET);
        arm::asm::dsb();
        unreachable!()
    }
}
