## Display controller firmware using STM32F429/Rust

This is an embedded display controller for a small TFT display
connected to the LTDC peripheral of a STM32F429.

It receives data over UART connection and handles two independent
framebuffers:

* a classic serial console (that supports ANSI sequences for
  graphics rendition and basic cursor movement), and
* a graphics view which can be drawn to using special escape
  sequences embedded in the console data stream.

Switching between the two framebuffers is also done via escape
sequences.

The use case is a noninteractive display for an otherwise headless
equipment control computer that shows bootup messages for debugging,
and then switches to a pretty display of the current status of the
connected equipment.

### Build

Using nightly Rust and cargo, just do a `cargo build --release`.

### Flash

The default method uses openocd and GDB.  Start openocd using a config
matching your programming adapter (the provided `openocd.cfg` assumes
ST-Link v2).  Then `cargo run --release` runs GDB, flashes and runs the
image.  `openocd` should just keep running in the background.

An alternate way is to use the `st-flash` utility.  To use this from
`cargo run --release`, change the runner in `.cargo/config`.
