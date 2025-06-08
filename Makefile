# make sure to source the following files before building/running:
# - source "${HOME}/git/esp/esp-idf/export.sh"
# - source "${HOME}/export-esp.sh"

SHELL := /bin/bash

ESP_IDF_DIR= $(shell dirname $$(dirname $$(command -v idf.py)))

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

.PHONY: erase-flash
erase-flash:
	esptool.py --chip esp32 erase_flash

.PHONY: build-ui
build-ui:
	(cd InterlockingArea-ui && npm run build)
	echo "Build timestamp: $$(date)" > InterlockingArea-ui/dist/build.ts

create-webapp-fs: build-ui
	$(ESP_IDF_DIR)/components/fatfs/fatfsgen.py \
		--sector_size 4096 \
		--partition_size $$((1024 * 1024)) \
		--output_file webapp_fs.bin \
		InterlockingArea-ui/dist

.PHONY: create-user-fs
create-user-fs:
	# 500kB - size of WL (wear-leveling) header
	$(ESP_IDF_DIR)/components/fatfs/fatfsgen.py \
		--sector_size 4096 \
		--partition_size $$((512 * 1024 - 4096)) \
		--output_file user_fs.bin \
		user_data

.PHONY: parse-fat-fs
parse-fat-fs:
	$(ESP_IDF_DIR)/components/fatfs/fatfsparse.py \
		webapp_fs.bin

wl-header.bin:
	# the wear-leveling (WL) header needs to be present before the actual FAT FS
	# as we are not using any wear-leveling, it consists of only 0xFF values
	dd if=/dev/zero bs=1 count=4096 | tr '\000' '\377' > $@

flash-user-fs: create-user-fs wl-header.bin
	# https://www.reddit.com/r/esp32/comments/plvaq3/comment/hcdmsir/
	esptool.py --chip esp32 -p /dev/ttyUSB0 -b 460800 \
		--before=default_reset --after=hard_reset \
		write_flash --flash_mode dio --flash_freq 40m --flash_size 4MB \
		0x310000 wl-header.bin \
		0x311000 user_fs.bin

flash-webapp-fs: create-webapp-fs
	# https://www.reddit.com/r/esp32/comments/plvaq3/comment/hcdmsir/
	esptool.py --chip esp32 -p /dev/ttyUSB0 -b 460800 \
		--before=default_reset --after=hard_reset \
		write_flash --flash_mode dio --flash_freq 40m --flash_size 4MB \
		0x210000 webapp_fs.bin

.PHONY: flash-all
flash-all:
	$(MAKE) erase-flash
	$(MAKE) flash-webapp-fs
	$(MAKE) flash-user-fs
	$(MAKE) run

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
