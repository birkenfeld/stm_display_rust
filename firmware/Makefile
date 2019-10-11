.PHONY: build install

CARGO = cargo

ELF = target/thumbv7em-none-eabihf/release/display_firmware

build:
	$(CARGO) build --release
	objcopy -Obinary $(ELF) display_fw.bin
	$(CARGO) build --release --features test-mode
	objcopy -Obinary $(ELF) display_fw_test.bin

install: build
	install -m 644 -Dt $(DESTDIR)/usr/share/display *.bin
