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
use hal::rcc::RccExt;
use hal::gpio::GpioExt;
use hal::flash::FlashExt;
use hal_base::prelude::*;

#[macro_use]
mod util;
mod font;

/// Width and height of visible screen.
const WIDTH: u16 = 480;
const HEIGHT: u16 = 128;

/// Size of a character.
const CHARH: u16 = 10;
const CHARW: u16 = 6;

/// Number of characters in the visible screen.
const COLS: u16 = WIDTH / CHARW;
const ROWS: u16 = HEIGHT / CHARH;

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

/// Default colors.
const DEFAULT_COLOR: u8 = 7;
const DEFAULT_BKGRD: u8 = 0;
const CURSOR_COLOR:  u8 = 127;

// Size of framebuffer: includes one extra row for scrolling via DMA
const FB_SIZE: usize = (WIDTH as usize) * ((HEIGHT + CHARH) as usize);

// main framebuffer
static mut FRAMEBUF: [u8; FB_SIZE] = [0; FB_SIZE];
// cursor framebuffer, just the cursor itself
static CURSORBUF: [u8; CHARW as usize] = [CURSOR_COLOR; CHARW as usize];

// TX receive buffer
static mut RXBUF: Option<ArrayDeque<[u8; 256]>> = None;

fn fifo() -> &'static mut ArrayDeque<[u8; 256]> {
    unsafe { RXBUF.get_or_insert_with(ArrayDeque::new) }
}

entry!(main);

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

    // set up pins
    let mut gpioa = peri.GPIOA.split(&mut rcc.ahb1);
    let mut gpiob = peri.GPIOB.split(&mut rcc.ahb1);
    let mut gpioc = peri.GPIOC.split(&mut rcc.ahb1);
    let mut gpiod = peri.GPIOD.split(&mut rcc.ahb1);
    let mut gpioe = peri.GPIOE.split(&mut rcc.ahb1);

    // LEDs
    let mut led1 = gpiob.pb7.into_push_pull_output(&mut gpiob.moder, &mut gpiob.otyper);
    let mut led2 = gpiob.pb14.into_push_pull_output(&mut gpiob.moder, &mut gpiob.otyper);

    led1.set_low();
    led2.set_high();

    // LCD_enable
    let mut disp_on = gpioa.pa8.into_push_pull_output(&mut gpioa.moder, &mut gpioa.otyper);
    disp_on.set_low();

    // set up blinking timer
    let mut timer = Timer::tim3(peri.TIM3, Hertz(4), clocks, &mut rcc.apb1);
    timer.listen(hal::timer::Event::TimeOut);

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
    write!(DMA2D.opfccr:  cm = 0b0101);

    // for scrolling up one line
    write!(DMA2D.fgmar: ma = FRAMEBUF.as_ptr().offset(CHARH as isize*WIDTH as isize) as u32);
    write!(DMA2D.fgor: lo = 0);
    write!(DMA2D.omar: ma = FRAMEBUF.as_ptr() as u32);
    write!(DMA2D.oor: lo = 0);
    write!(DMA2D.nlr: pl = WIDTH, nl = HEIGHT);

    // Configure LCD timings
    write!(LTDC.sscr: hsw = H_SYNCPULSE - 1, vsh = V_SYNCPULSE - 1); // -1 required by STM
    write!(LTDC.bpcr: ahbp = H_WIN_START, avbp = V_WIN_START);
    write!(LTDC.awcr: aaw = H_WIN_START + H_ACTIVE, aah = V_WIN_START + V_ACTIVE);
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
    write!(LTDC.l1cfbar: cfbadd = FRAMEBUF.as_ptr() as u32);
    // Color frame buffer line length (active*bpp + 3), and pitch
    write!(LTDC.l1cfblr: cfbll = WIDTH + 3, cfbp = WIDTH);
    // Frame buffer number of lines
    write!(LTDC.l1cfblnr: cfblnbr = HEIGHT);

    // Set up 256-color ANSI LUT
    for v in 0..16 {
        let b = (v & 4 != 0) as u8;
        let g = (v & 2 != 0) as u8;
        let r = (v & 1 != 0) as u8;
        let i = (v & 8 != 0) as u8;
        write!(LTDC.l1clutwr: clutadd = v,
               red = 0x55*(r<<1 | i), green = 0x55*(g<<1 | i), blue = 0x55*(b<<1 | i));
    }
    for r in 0..6 {
        for g in 0..6 {
            for b in 0..6 {
                write!(LTDC.l1clutwr: clutadd = 16 + 36*b + 6*g + r,
                       red = 0x33*r, green = 0x33*g, blue = 0x33*b);
            }
        }
    }
    for i in 0..24 {
        write!(LTDC.l1clutwr: clutadd = 232+i, red = 8+10*i, green = 8+10*i, blue = 8+10*i);
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
    nvic.enable(stm::Interrupt::USART3);

    // indicate readiness
    led1.set_high();
    led2.set_low();

    draw(COLS-3, 1, b'O', 0b1010, 0b1100);
    draw(COLS-2, 1, b'K', 0b1010, 0b1100);

    unsafe {
        for &y in &[0, HEIGHT-1] {
            for x in 0..WIDTH {
                FRAMEBUF[x as usize + (y * WIDTH) as usize] = 0xff;
            }
        }
        for &x in &[0, WIDTH-1] {
            for y in 0..HEIGHT {
                FRAMEBUF[x as usize + (y * WIDTH) as usize] = 0xff;
            }
        }
    }

    main_loop(console_tx)
}

fn cursor(cx: u16, cy: u16) {
    write!(LTDC.l2whpcr: whstpos = H_WIN_START + cx*CHARW + 1,
           whsppos = H_WIN_START + (cx + 1)*CHARW);
    write!(LTDC.l2wvpcr: wvstpos = V_WIN_START + (cy + 1)*CHARH,
           wvsppos = V_WIN_START + (cy + 1)*CHARH);
    // reload on next vsync
    write!(LTDC.srcr: vbr = true);
}

fn draw(cx: u16, cy: u16, ch: u8, color: u8, bkgrd: u8) {
    font::FONT[ch as usize].iter().zip(cy*CHARH..(cy+1)*CHARH).for_each(|(charrow, y)| {
        (0..CHARW).for_each(|x| unsafe {
            FRAMEBUF[(x + cx*CHARW) as usize + (y * WIDTH) as usize] =
                if charrow & (1 << (CHARW - 1 - x)) != 0 { color } else { bkgrd };
        });
    });
}

fn process_escape(end: u8, seq: &[u8], cx: &mut u16, cy: &mut u16, color: &mut u8, bkgrd: &mut u8) {
    let mut args = seq.split(|&v| v == b';').map(|n| btoi(n).unwrap_or(0));
    match end {
        b'm' => while let Some(arg) = args.next() {
            match arg {
                0  => { *color = DEFAULT_COLOR; *bkgrd = DEFAULT_BKGRD; }
                1  => { *color |= 0b1000; } // XXX: only for 16colors
                22 => { *color &= !0b1000; }
                30...37 => { *color = arg as u8 - 30; }
                40...47 => { *bkgrd = arg as u8 - 40; }
                38 => { *color = args.nth(1).unwrap_or(0) as u8; }
                48 => { *bkgrd = args.nth(1).unwrap_or(0) as u8; }
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
