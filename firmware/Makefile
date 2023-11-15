.PHONY: build install

CARGO = cargo
OBJCOPY = objcopy

ELF = target/thumbv7em-none-eabihf/release/display_firmware

build:
	$(CARGO) build --release --features test-mode
	$(OBJCOPY) -Obinary $(ELF) display_fw_test.bin
	$(CARGO) build --release
	$(OBJCOPY) -Obinary $(ELF) display_fw.bin

install: build
	install -m 644 -Dt $(DESTDIR)/usr/share/display *.bin

psi:
	$(CARGO) build --release --no-default-features --features customer-psi
	$(OBJCOPY) -Obinary $(ELF) display_fw_psi.bin
fau:
	$(CARGO) build --release --no-default-features --features customer-fau
	$(OBJCOPY) -Obinary $(ELF) display_fw_fau.bin
