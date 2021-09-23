.PHONY: all release release-patch release-minor

all:
	$(MAKE) -C firmware
	$(MAKE) -C simulator

versionbump:
	for p in lib firmware simulator; do (cd $$p; cargo set-version 1.$(VER).0); done

release-minor:
	MODE="minor" $(MAKE) release

release:
	ssh jenkins.admin.frm2 -p 29417 build -v -s -p GERRIT_PROJECT=$(shell git config --get remote.origin.url | rev | cut -d '/' -f -3 | rev) -p ARCH=any -p MODE=$(MODE) ReleasePipeline
