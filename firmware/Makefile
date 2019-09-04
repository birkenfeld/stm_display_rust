.PHONY: build install

CARGO = cargo

build:
	$(CARGO) build --release

install: build
	objcopy -Obinary target/thumbv7em-none-eabihf/release/display_firmware display_fw.bin
	install -m 644 -D display_fw.bin $(DESTDIR)/usr/share/display/firmware.bin
