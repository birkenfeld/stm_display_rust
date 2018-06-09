#![no_main]
#![no_std]

#[macro_use]
extern crate cortex_m_rt as rt;
extern crate cortex_m_semihosting as sh;
extern crate panic_semihosting;
#[macro_use]
extern crate nb;
extern crate btoi;
extern crate arraydeque;
extern crate embedded_hal as hal_base;
extern crate cortex_m as arm;
#[macro_use]
extern crate stm32f429 as stm;
extern crate stm32f429_hal as hal;

use rt::ExceptionFrame;
use btoi::btoi;
use arraydeque::ArrayDeque;
use hal::time::*;
use hal::timer::Timer;
use hal::delay::Delay;
use hal::rcc::RccExt;
use hal::gpio::GpioExt;
use hal::flash::FlashExt;
use hal_base::spi;
use hal_base::prelude::*;

#[macro_use]
mod util;
mod font;

entry!(main);

const ILI9341_RESET: u8 = 0x01;
const ILI9341_SLEEP_OUT: u8 = 0x11;
const ILI9341_DISPLAY_ON: u8 = 0x29;
const ILI9341_MAC: u8 = 0x36;
// const ILI9341_PIXEL_FORMAT: u8 = 0x3A;
const ILI9341_RGB_INTERFACE: u8 = 0xB0;
const ILI9341_INTERFACE: u8 = 0xF6;

const WIDTH: usize = 320;
const HEIGHT: usize = 240;
const PITCH: usize = 250;
const COLS: u16 = 53;
const ROWS: u16 = 24;
const CHARH: u16 = 10;
const CHARW: u16 = 6;
const DEFAULT_COLOR: u8 = 7;
const DEFAULT_BKGRD: u8 = 0;

// main framebuffer
static mut FRAMEBUF: [u8; 250*320] = [0; 250*320];
// cursor framebuffer, just the cursor itself
static CURSORBUF: [u8; 6] = [127; 6];

// TX receive buffer
static mut RXBUF: Option<ArrayDeque<[u8; 256]>> = None;

fn fifo() -> &'static mut ArrayDeque<[u8; 256]> {
    unsafe { RXBUF.get_or_insert_with(ArrayDeque::new) }
}

fn main() -> ! {
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
    let mut time = Delay::new(pcore.SYST, clocks);

    // set up pins
    let mut gpioa = peri.GPIOA.split(&mut rcc.ahb1);
    let mut gpiob = peri.GPIOB.split(&mut rcc.ahb1);
    let mut gpioc = peri.GPIOC.split(&mut rcc.ahb1);
    let mut gpiod = peri.GPIOD.split(&mut rcc.ahb1);
    let mut gpiof = peri.GPIOF.split(&mut rcc.ahb1);
    let mut gpiog = peri.GPIOG.split(&mut rcc.ahb1);

    // LEDs
    let mut led1 = gpiog.pg13.into_push_pull_output(&mut gpiog.moder, &mut gpiog.otyper);
    let mut led2 = gpiog.pg14.into_push_pull_output(&mut gpiog.moder, &mut gpiog.otyper);

    led1.set_low();
    led2.set_high();

    // set up blinking timer
    let mut timer = Timer::tim3(peri.TIM3, Hertz(4), clocks, &mut rcc.apb1);
    timer.listen(hal::timer::Event::TimeOut);

    // LCD SPI
    let mut cs = gpioc.pc2.into_push_pull_output(&mut gpioc.moder, &mut gpioc.otyper);
    let mut ds = gpiod.pd13.into_push_pull_output(&mut gpiod.moder, &mut gpiod.otyper);
    let sclk = gpiof.pf7.into_af5(&mut gpiof.moder, &mut gpiof.afrl);
    let miso = gpiof.pf8.into_af5(&mut gpiof.moder, &mut gpiof.afrl);
    let mosi = gpiof.pf9.into_af5(&mut gpiof.moder, &mut gpiof.afrh);
    let mut display_spi = hal::spi::Spi::spi5(peri.SPI5, (sclk, miso, mosi),
        spi::Mode { polarity: spi::Polarity::IdleLow, phase: spi::Phase::CaptureOnFirstTransition },
        MegaHertz(1), clocks, &mut rcc.apb2);

    // Console UART (UART #3)
    let utx = gpiod.pd8 .into_af7(&mut gpiod.moder, &mut gpiod.afrh);
    let urx = gpiod.pd9 .into_af7(&mut gpiod.moder, &mut gpiod.afrh);
    let rts = gpiod.pd12.into_af7(&mut gpiod.moder, &mut gpiod.afrh);
    let mut console_uart = hal::serial::Serial::usart3(peri.USART3, (utx, urx),
        hal::time::Bps(115200), clocks, &mut rcc.apb1);
    console_uart.set_rts(rts);
    console_uart.listen(hal::serial::Event::Rxne);
    let (console_tx, _) = console_uart.split();

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
    gpiof.pf10.into_lcd(&mut gpiof.moder, &mut gpiof.ospeedr, &mut gpiof.afrh, 0xE);
    gpiog.pg6 .into_lcd(&mut gpiog.moder, &mut gpiog.ospeedr, &mut gpiog.afrl, 0xE);
    gpiog.pg7 .into_lcd(&mut gpiog.moder, &mut gpiog.ospeedr, &mut gpiog.afrl, 0xE);
    gpiog.pg10.into_lcd(&mut gpiog.moder, &mut gpiog.ospeedr, &mut gpiog.afrh, 0x9);
    gpiog.pg11.into_lcd(&mut gpiog.moder, &mut gpiog.ospeedr, &mut gpiog.afrh, 0xE);
    gpiog.pg12.into_lcd(&mut gpiog.moder, &mut gpiog.ospeedr, &mut gpiog.afrh, 0x9);

    // enable clocks
    modif!(RCC.apb2enr: ltdcen = true);
    modif!(RCC.ahb1enr: dma2den = true);
    // enable PLLSAI
    // PLLSAI_VCO Input = HSE_VALUE/PLL_M = 1 Mhz
    // PLLSAI_VCO Output = PLLSAI_VCO Input * PLLSAI_N = 192 Mhz
    // PLLLCDCLK = PLLSAI_VCO Output/PLLSAI_R = 192/4 = 96 Mhz
    // LTDC clock frequency = PLLLCDCLK / RCC_PLLSAIDivR = 96/4 = 24 Mhz
    write!(RCC.pllsaicfgr: pllsain = 192, pllsaiq = 7, pllsair = 4);
    write!(RCC.dckcfgr: pllsaidivr = 0b01);  // divide by 4
    // enable PLLSAI and wait for it
    modif!(RCC.cr: pllsaion = true);
    wait_for!(RCC.cr: pllsairdy);

    // Basic ChromArt configuration
    write!(DMA2D.fgpfccr: cm = 0b0101);  // L8 in/out
    write!(DMA2D.opfccr:  cm = 0b0101);

    // for scrolling up one line
    write!(DMA2D.fgmar: ma = FRAMEBUF.as_ptr().offset(CHARH as isize) as u32);
    write!(DMA2D.fgor: lo = CHARH);
    write!(DMA2D.omar: ma = FRAMEBUF.as_ptr() as u32);
    write!(DMA2D.oor: lo = CHARH);
    write!(DMA2D.nlr: pl = HEIGHT as u16, nl = WIDTH as u16);

    // Configure LCD timings
    write!(LTDC.sscr: hsw = 9, vsh = 1);            // Vsync, Hsync
    write!(LTDC.bpcr: ahbp = 29, avbp = 3);         // Back porch
    write!(LTDC.awcr: aaw = 269, aah = 323);        // Active width
    write!(LTDC.twcr: totalw = 279, totalh = 327);  // Total width

    // Configure layer 1 (main framebuffer)

    // Horizontal and vertical window (coordinates include porches)
    write!(LTDC.l1whpcr: whstpos = 30, whsppos = 269);
    write!(LTDC.l1wvpcr: wvstpos = 4,  wvsppos = 323);
    // Pixel format
    write!(LTDC.l1pfcr: pf = 0b101);  // 8-bit (CLUT enabled below)
    // Constant alpha value
    write!(LTDC.l1cacr: consta = 0xFF);
    // Default color values
    write!(LTDC.l1dccr:  dcalpha = 0, dcred = 0, dcgreen = 0, dcblue = 0);
    // Blending factors
    write!(LTDC.l1bfcr: bf1 = 4, bf2 = 5);  // Constant alpha
    // Color frame buffer start address
    write!(LTDC.l1cfbar: cfbadd = FRAMEBUF.as_ptr() as u32);
    // Color frame buffer line length (active*bpp + 3), and pitch
    write!(LTDC.l1cfblr: cfbll = 240 + 3, cfbp = 250);
    // Frame buffer number of lines
    write!(LTDC.l1cfblnr: cfblnbr = 320);

    // Set up 256-color ANSI LUT
    for v in 0..16 {
        let b = (v & 1 != 0) as u8;
        let g = (v & 2 != 0) as u8;
        let r = (v & 4 != 0) as u8;
        let i = (v & 8 != 0) as u8;
        write!(LTDC.l1clutwr: clutadd = v,
               red = 0x55*(r<<1 | i), green = 0x55*(g<<1 | i), blue = 0x55*(b<<1 | i));
    }
    for r in 0..6 {
        for g in 0..6 {
            for b in 0..6 {
                write!(LTDC.l1clutwr: clutadd = 16 + 36*r + 6*g + b,
                       red = 0x33*r, green = 0x33*g, blue = 0x33*b);
            }
        }
    }
    for i in 0..24 {
        write!(LTDC.l1clutwr: clutadd = 232+i, red = 11*i, green = 11*i, blue = 11*i);
    }

    // Configure layer 2 (cursor)

    // initial position: top left character
    write!(LTDC.l2whpcr: whstpos = 30+9, whsppos = 30+9);
    write!(LTDC.l2wvpcr: wvstpos = 4,  wvsppos = 4+6-1);
    write!(LTDC.l2pfcr: pf = 0b101);  // L-8 without CLUT
    write!(LTDC.l2cacr: consta = 0xFF);
    write!(LTDC.l2dccr: dcalpha = 0, dcred = 0, dcgreen = 0, dcblue = 0);
    write!(LTDC.l2bfcr: bf1 = 6, bf2 = 7);  // Constant alpha * Pixel alpha
    write!(LTDC.l2cfbar: cfbadd = CURSORBUF.as_ptr() as u32);
    write!(LTDC.l2cfblr: cfbll = 1 + 3, cfbp = 1);
    write!(LTDC.l2cfblnr: cfblnbr = 6);

    // Enable layer1, disable layer2 initially
    modif!(LTDC.l1cr: cluten = true, len = true);
    modif!(LTDC.l2cr: len = false);

    // Reload config again
    write!(LTDC.srcr: imr = true);  // Immediate reload

    // Dither on, display on
    modif!(LTDC.gcr: den = true, ltdcen = true);

    // Reload config to show display
    write!(LTDC.srcr: imr = true);

    // Initialize LCD controller
    cs.set_high();
    spi_cmd!(display_spi, time, cs, ds, ILI9341_RESET);
    time.delay_ms(5u16);
    spi_cmd!(display_spi, time, cs, ds, ILI9341_MAC, 0xC0);
    spi_cmd!(display_spi, time, cs, ds, ILI9341_RGB_INTERFACE, 0xC2);
    spi_cmd!(display_spi, time, cs, ds, ILI9341_INTERFACE, 0x01, 0x00, 0x06);
    spi_cmd!(display_spi, time, cs, ds, ILI9341_SLEEP_OUT);
    time.delay_ms(60u16);
    spi_cmd!(display_spi, time, cs, ds, ILI9341_DISPLAY_ON);

    // enable interrupts
    let mut nvic = pcore.NVIC;
    nvic.enable(stm::Interrupt::TIM3);
    nvic.enable(stm::Interrupt::USART3);

    // indicate readiness
    led1.set_high();
    led2.set_low();

    draw(COLS-3, 1, b'O', 0b1010, 0b1100);
    draw(COLS-2, 1, b'K', 0b1010, 0b1100);

    main_loop(console_tx)
}

fn cursor(cx: u16, cy: u16) {
    write!(LTDC.l2whpcr: whstpos = 30 + 9 + cy*CHARH, whsppos = 30 + 9 + cy*CHARH);
    write!(LTDC.l2wvpcr: wvstpos = 4 + cx*CHARW, wvsppos = 4 + 6 - 1 + cx*CHARW);
    // reload on next vsync
    write!(LTDC.srcr: vbr = true);
}

fn draw(cx: u16, cy: u16, ch: u8, color: u8, bkgrd: u8) {
    font::FONT[ch as usize].iter().zip(cy*CHARH..(cy+1)*CHARH).for_each(|(charrow, y)| {
        (0..CHARW).for_each(|x| unsafe {
            FRAMEBUF[(x + cx*CHARW) as usize * PITCH + y as usize] =
                if charrow & (1 << (CHARW - 1 - x)) != 0 { color } else { bkgrd };
        });
    });
}

fn process_escape(end: u8, seq: &[u8], cx: &mut u16, cy: &mut u16, color: &mut u8, bkgrd: &mut u8) {
    let mut args = seq.split(|&v| v == b';').map(|n| btoi(n).unwrap_or(0));
    match end {
        b'm' => for arg in args {
            match arg {
                0  => { *color = DEFAULT_COLOR; *bkgrd = DEFAULT_BKGRD; }
                1  => { *color |= 0b1000; } // XXX: only for 16colors
                22 => { *color &= !0b1000; }
                30...37 => { *color = arg as u8 - 30; }
                40...47 => { *bkgrd = arg as u8 - 40; }
                _ => {}
            }
        },
        b'H' | b'f' => {
            let y = args.next().unwrap_or(1);
            let x = args.next().unwrap_or(1);
            *cx = if x > 0 { x-1 } else { 0 };
            *cy = if y > 0 { y-1 } else { 0 };
        },
        b'A' => {
            let n = args.next().unwrap_or(1).max(1);
            *cy -= n.min(*cy);
        },
        b'B' => {
            let n = args.next().unwrap_or(1).max(1);
            *cy += n.min(ROWS - *cy - 1);
        },
        b'C' => {
            let n = args.next().unwrap_or(1).max(1);
            *cx += n.min(COLS - *cx - 1);
        },
        b'D' => {
            let n = args.next().unwrap_or(1).max(1);
            *cx -= n.min(*cx);
        },
        b'G' => {
            let x = args.next().unwrap_or(1).max(1);
            *cx = x-1;
        }
        b'J' => {}, // TODO: erase screen
        b'K' => {}, // TODO: erase line
        // otherwise, ignore
        _    => {}
    }
}

fn main_loop(mut console_tx: hal::serial::Tx<stm::USART3>) -> ! {
    let mut cx = 0;
    let mut cy = 0;
    let mut color = DEFAULT_COLOR;
    let mut bkgrd = DEFAULT_BKGRD;
    let mut escape = 0;
    let mut escape_len = 0;
    let mut escape_seq = [0u8; 36];

    loop {
        if let Some(ch) = fifo().pop_front() {
            block!(console_tx.write(ch)).unwrap();

            if escape == 1 {
                escape_len = 0;
                escape = if ch == b'[' { 2 } else { 0 };
                continue;
            } else if escape == 2 {
                if (ch >= b'0' && ch <= b'9') || ch == b';' {
                    escape_seq[escape_len] = ch;
                    escape_len += 1;
                    if escape_len == escape_seq.len() {
                        escape = 0;
                    }
                } else {
                    process_escape(ch, &escape_seq[..escape_len],
                                   &mut cx, &mut cy, &mut color, &mut bkgrd);
                    cursor(cx, cy);
                    escape = 0;
                }
                continue;
            }

            if ch == b'\r' {
                // do nothing
            } else if ch == b'\n' {
                cx = 0;
                cy += 1;
                if cy == ROWS {
                    // scroll one row using DMA
                    modif!(DMA2D.cr: mode = 0, start = true);
                    wait_for!(DMA2D.cr: start);
                    cy -= 1;
                }
                cursor(cx, cy);
            } else if ch == b'\x08' {
                if cx > 0 {
                    cx -= 1;
                    draw(cx, cy, b' ', color, bkgrd);
                    cursor(cx, cy);
                }
            } else if ch == b'\x1b' {
                escape = 1;
            } else {
                draw(cx, cy, ch, color, bkgrd);
                cx = (cx + 1) % COLS;
                cursor(cx, cy);
            }
        }
    }
}

interrupt!(TIM3, blink, state: bool = false);

fn blink(visible: &mut bool) {
    // toggle layer2 on next vsync
    *visible = !*visible;
    modif!(LTDC.l2cr: len = bit(*visible));
    write!(LTDC.srcr: vbr = true);
    // reset timer
    modif!(TIM3.sr: uif = false);
    modif!(TIM3.cr1: cen = true);
}

interrupt!(USART3, receive);

fn receive() {
    let data = read!(USART3.dr: dr) as u8;
    let _ = fifo().push_back(data);
}

exception!(HardFault, hard_fault);

fn hard_fault(ef: &ExceptionFrame) -> ! {
    panic!("HardFault at {:#?}", ef);
}

exception!(*, default_handler);

fn default_handler(irqn: i16) {
    panic!("Unhandled exception (IRQn = {})", irqn);
}
