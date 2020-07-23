.PHONY: build install

CARGO = cargo

build:
	$(CARGO) build --release

install: build
	install -m 755 -D target/release/display_simulator $(DESTDIR)/usr/bin/display-simulator
