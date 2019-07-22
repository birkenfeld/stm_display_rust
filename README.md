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

Using a recent Rust compiler, just do a `cargo build --release` in
the `firmware` directory.

### Flash

The default method uses openocd and GDB.  Start openocd using a config
matching your programming adapter (the provided `openocd.cfg` assumes
ST-Link v2).  Then `cargo run --release` runs GDB, flashes and runs
the image.  `openocd` should just keep running in the background.

An alternate way is to use the `st-flash` utility, as shown in
`run-alternate`.  To use this from `cargo run --release`, change the
runner in `.cargo/config`.

### Simulator

The directory `simulator` contains a standalone Rust program that
simulates the display controller.  Almost all of the code is shared
with the actual display firmware.

The simulator opens a pseudo-terminal that should be used instead of
the serial terminal for the display.

To run the demo on the simulator, do the following:

```
(cd simulator; cargo run --release) &
util/demo.py /dev/pts/X
```

where `X` is the PTS device number that is printed by the display
simulator.
