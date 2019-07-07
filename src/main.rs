#![no_main]
#![no_std]

extern crate panic_semihosting;

use stm32f4::stm32f429 as stm;
use stm::interrupt;
use cortex_m::{asm, interrupt as interrupts};
use cortex_m_rt::ExceptionFrame;
use heapless::mpmc::{Q16, Q64};
use hal::time::*;
use hal::timer::{Timer, Event};
use hal::serial::{Serial, config::Config as SerialConfig};
use hal::rcc::RccExt;
use hal::gpio::{GpioExt, Speed};
use embedded_hal::digital::v2::{InputPin, OutputPin};
use core::sync::atomic::{AtomicBool, Ordering};

#[macro_use]
mod regutil;
mod icon;
mod i2ceeprom;
mod spiflash;
mod interface;
mod framebuf;
mod console;
mod test_mode;
mod konami_mode;

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
static CURSOR_ENABLED: AtomicBool = AtomicBool::new(false);

// UART receive buffer
static UART_RX: Q64<u8> = Q64::new();

// Touch event buffer
static TOUCH_EVT: Q16<u16> = Q16::new();

#[cortex_m_rt::entry]
fn main() -> ! {
    // let mut stdout = hio::hstdout().unwrap();
    let pcore = cortex_m::Peripherals::take().unwrap();
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
    let _ = disp_on.set_low();

    // LCD backlight enable
    let mut backlight = gpiod.pd12.into_push_pull_output();
    let _ = backlight.set_high();

    // Output pin connected to Boot0 + capacitor
    let mut bootpin = gpiob.pb7.into_push_pull_output();
    let _ = bootpin.set_low();

    // Set up blinking timer
    let mut blink_timer = Timer::tim3(peri.TIM3, Hertz(4), clocks);

    // Set up touch detection timer
    let mut touch_timer = Timer::tim4(peri.TIM4, Hertz(100), clocks);

    // External Flash memory via SPI
    let cs = gpiob.pb12.into_push_pull_output();
    let sclk = gpiob.pb13.into_alternate_af5();
    let miso = gpiob.pb14.into_alternate_af5();
    let mosi = gpiob.pb15.into_alternate_af5();
    let spi2 = hal::spi::Spi::spi2(peri.SPI2, (sclk, miso, mosi),
        embedded_hal::spi::MODE_0, Hertz(40_000_000), clocks);
    let mut spi_flash = spiflash::SPIFlash::new(spi2, cs);

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

    // Extension header pins
    let testmode_pin = gpioe.pe1.into_pull_down_input();
    gpioe.pe0.into_pull_down_input();
    gpiob.pb6.into_pull_down_input();
    gpiob.pb5.into_pull_down_input();
    gpiob.pb4.into_pull_down_input();
    gpiob.pb3.into_pull_down_input();
    gpiod.pd7.into_pull_down_input();
    gpiod.pd5.into_pull_down_input();
    gpiod.pd4.into_pull_down_input();
    gpiod.pd2.into_pull_down_input();
    gpiod.pd1.into_pull_down_input();
    gpiod.pd0.into_pull_down_input();
    gpioc.pc12.into_pull_down_input();
    gpioc.pc11.into_pull_down_input();

    // Pins for touch screen
    let _touch_yd = gpioc.pc0.into_pull_down_input();
    let _touch_yu = gpioc.pc1.into_floating_input();  // LATER: analog
    let mut touch_xl = gpioc.pc2.into_push_pull_output();
    let _ = touch_xl.set_low();
    let mut touch_xr = gpioc.pc3.into_push_pull_output();
    let _ = touch_xr.set_high();

    // Pin for resetting the APU
    let mut reset_pin = gpioc.pc8.into_open_drain_output();
    let _ = reset_pin.set_high();

    // Set yu input pin to analog mode.  Hardcoded for now!
    modif!(GPIOC.moder: moder1 = @analog);

    // Activate and configure ADC.
    modif!(RCC.apb2enr: adc1en = true);
    pulse!(RCC.apb2rstr: adcrst);
    // One conversion of channel 11, continuous mode.
    write!(ADC1.sqr1: l = 0);
    write!(ADC1.sqr3: sq1 = 11);
    write!(ADC1.cr1: awden = true);
    write!(ADC1.cr2: cont = true, adon = true);
    modif!(ADC1.cr2: swstart = true);

    // Enable clocks
    modif!(RCC.apb2enr: ltdcen = true);
    pulse!(RCC.apb2rstr: ltdcrst);
    modif!(RCC.ahb1enr: dma2den = true);
    pulse!(RCC.ahb1rstr: dma2drst);
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
    write!(LTDC.layer1.whpcr: whstpos = H_WIN_START + 1, whsppos = H_WIN_START + WIDTH);
    write!(LTDC.layer1.wvpcr: wvstpos = V_WIN_START + 1, wvsppos = V_WIN_START + HEIGHT);
    // Pixel format
    write!(LTDC.layer1.pfcr: pf = 0b101);  // 8-bit (CLUT enabled below)
    // Constant alpha value
    write!(LTDC.layer1.cacr: consta = 0xFF);
    // Default color values
    write!(LTDC.layer1.dccr: dcalpha = 0, dcred = 0, dcgreen = 0, dcblue = 0);
    // Blending factors
    write!(LTDC.layer1.bfcr: bf1 = 4, bf2 = 5);  // Constant alpha
    // Color frame buffer start address
    write!(LTDC.layer1.cfbar: cfbadd = FB_CONSOLE.as_ptr() as u32);
    // Color frame buffer line length (active*bpp + 3), and pitch
    write!(LTDC.layer1.cfblr: cfbll = WIDTH + 3, cfbp = WIDTH);
    // Frame buffer number of lines
    write!(LTDC.layer1.cfblnr: cfblnbr = HEIGHT);
    // Set up 256-color LUT
    for (i, (r, g, b)) in console::get_lut_colors().enumerate() {
        write!(LTDC.layer1.clutwr: clutadd = i as u8, red = r, green = g, blue = b);
    }

    // Configure layer 2 (cursor)

    // Initial position: top left character
    write!(LTDC.layer2.whpcr: whstpos = H_WIN_START + 1, whsppos = H_WIN_START + CHARW);
    write!(LTDC.layer2.wvpcr: wvstpos = V_WIN_START + CHARH, wvsppos = V_WIN_START + CHARH);
    write!(LTDC.layer2.pfcr: pf = 0b101);  // L-8 without CLUT
    write!(LTDC.layer2.cacr: consta = 0xFF);
    write!(LTDC.layer2.dccr: dcalpha = 0, dcred = 0, dcgreen = 0, dcblue = 0);
    write!(LTDC.layer2.bfcr: bf1 = 6, bf2 = 7);  // Constant alpha * Pixel alpha
    write!(LTDC.layer2.cfbar: cfbadd = CURSORBUF.as_ptr() as u32);
    write!(LTDC.layer2.cfblr: cfbll = CHARW + 3, cfbp = CHARW);
    write!(LTDC.layer2.cfblnr: cfblnbr = 1);  // Cursor is one line of 6 pixels

    // Enable layer1, disable layer2 initially
    modif!(LTDC.layer1.cr: cluten = true, len = true);
    modif!(LTDC.layer2.cr: len = false);

    // Reload config (immediate)
    write!(LTDC.srcr: imr = true);

    // Dither on, display on
    modif!(LTDC.gcr: den = true, ltdcen = true);

    // Reload config to show display
    write!(LTDC.srcr: imr = true);

    // Enable display via GPIO too
    let _ = disp_on.set_high();

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

    // Activate USART receiver
    modif!(USART1.cr1: rxneie = true);
    nvic.enable(stm::Interrupt::USART1);

    if testmode_pin.is_high().unwrap() {
        test_mode::run(&mut disp, &mut spi_flash, &mut eeprom);
    }

    // Switch to console if nothing else programmed
    disp.console().activate();

    // Load pre-programmed startup sequence from EEPROM
    let mut startup_buf = [0; 256];
    if let Ok(code) = eeprom.read_stored_entry(0, 64, &mut startup_buf) {
        for &byte in code {
            // TODO: might not fit into 64 bytes!
            let _ = UART_RX.enqueue(byte);
        }
    }

    /* -- Touch calibration mode --
    let (gfx, _) = disp.split();
    gfx.activate();
    gfx.clear(15);
    loop {
        let (x, _) = wait_touch();
        gfx.rect(x - 1, 0, x + 2, 128, 4);
    }
    */

    let mut touch_ring = wheelbuf::WheelBuf::new([0u16; 8]);

    // Normal main loop: process input from UART
    loop {
        if let Some(ev) = TOUCH_EVT.dequeue() {
            let (x, _) = disp.process_touch(ev);
            touch_ring.push(x / 120);
            if touch_ring.iter().eq(konami_mode::ACTIVATION) {
                konami_mode::run(&mut disp, &mut reset_pin);
            }
        }
        if let Some(ch) = UART_RX.dequeue() {
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

fn recv_uart() -> u8 {
    loop {
        if let Some(ch) = UART_RX.dequeue() {
            return ch;
        }
        asm::wfi();
    }
}

fn clear_uart() {
    while UART_RX.dequeue().is_some() {}
}

pub fn enable_cursor(en: bool) {
    CURSOR_ENABLED.store(en, Ordering::Relaxed);
}

#[stm::interrupt]
fn TIM3() {
    static mut VISIBLE: bool = false;
    // Toggle layer2 on next vsync
    *VISIBLE = !*VISIBLE;
    modif!(LTDC.layer2.cr: len = bit(CURSOR_ENABLED.load(Ordering::Relaxed) && *VISIBLE));
    write!(LTDC.srcr: vbr = true);
    // Reset timer
    modif!(TIM3.sr: uif = false);
    modif!(TIM3.cr1: cen = true);
}

#[stm::interrupt]
fn USART1() {
    let data = read!(USART1.dr: dr) as u8;
    let _ = UART_RX.enqueue(data);
}

const THRESHOLD: u16 = 500;
const NSAMPLES: usize = 8;

struct TouchState {
    last: bool,
    data: [u16; NSAMPLES],
    idx: usize,
}

#[stm::interrupt]
fn TIM4() {
    static mut STATE: TouchState = TouchState { last: false, data: [0; NSAMPLES], idx: 0};
    let data = read!(ADC1.dr: data);
    STATE.data[STATE.idx] = data;
    STATE.idx = (STATE.idx + 1) % NSAMPLES;
    let mini = STATE.data.iter().cloned().min().unwrap();
    if !STATE.last && mini > THRESHOLD {
        let mean = STATE.data.iter().map(|&v| v as u32).sum::<u32>() / NSAMPLES as u32;
        let _ = TOUCH_EVT.enqueue(mean as u16);
        STATE.last = true;
    } else if STATE.last && mini < THRESHOLD {
        STATE.last = false;
    }
    // Reset timer
    modif!(TIM4.sr: uif = false);
    modif!(TIM4.cr1: cen = true);
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
        interrupts::disable();
        asm::dsb();
        // Do a soft-reset of the cpu
        scb.aircr.write(SCB_AIRCR_RESET);
        asm::dsb();
        unreachable!()
    }
}

pub fn reset_to_bootloader<O: OutputPin>(scb: stm::SCB, mut pin: O) -> ! {
    unsafe {
        interrupts::disable();
        // Set Boot0 high (keeps high through reset via RC circuit)
        let _ = pin.set_high();
        asm::delay(10000);
        asm::dsb();
        scb.aircr.write(SCB_AIRCR_RESET);
        asm::dsb();
        unreachable!()
    }
}
