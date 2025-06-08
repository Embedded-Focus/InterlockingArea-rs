# make sure to source the following files before building/running:
# - source "${HOME}/git/esp/esp-idf/export.sh"
# - source "${HOME}/export-esp.sh"

SHELL := /bin/bash

all: build-dev

.PHONY: build-dev  # defer all dependency handling to rust/cargo
build-dev:
	cargo +esp build --profile dev --target xtensa-esp32-espidf

.PHONY: build-release  # defer all dependency handling to rust/cargo
build-release:
	cargo +esp build --release --target xtensa-esp32-espidf

.PHONY: clippy  # defer all dependency handling to rust/cargo
clippy:
	cargo +esp clippy

.PHONY: run  # defer all dependency handling to rust/cargo
run:
	cargo +esp run --profile dev --target xtensa-esp32-espidf

.PHONY: monitor
monitor:
	espflash monitor

# usbip
# - https://www.unifix.org/2023/11/28/usbip-on-debian-12-usb-device-sharing-over-ip-network/

# list remote devices
.PHONY: usb-list
usb-list:
	sudo usbip list -r raspberry-dev

# list remote devices which are locally bound
.PHONY: usb-ports
usb-ports:
	sudo usbip port

.PHONY: usb-attach
usb-attach:
	sudo usbip attach -r raspberry-dev -b 1-1.1

.PHONY: usb-detach
usb-detach:
	sudo usbip detach -p 00
