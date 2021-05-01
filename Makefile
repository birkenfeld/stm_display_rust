.PHONY: all release release-patch release-minor

all:
	$(MAKE) -C firmware
	$(MAKE) -C simulator

versionbump:
	sed -i -e 's,^version = "1\.$(shell echo $$(($(VER) - 1)))\.0",version = "1.'$(VER)'.0",' lib/Cargo.toml firmware/Cargo.toml simulator/Cargo.toml

release-minor:
	MODE="minor" $(MAKE) release

release:
	ssh jenkins.admin.frm2 -p 29417 build -v -s -p GERRIT_PROJECT=$(shell git config --get remote.origin.url | rev | cut -d '/' -f -3 | rev) -p ARCH=any -p MODE=$(MODE) ReleasePipeline
