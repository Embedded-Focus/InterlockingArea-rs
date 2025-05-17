all: build-dev

.PHONY: build-dev  # defer all dependency handling to rust/cargo
build-dev:
	cargo +esp build --profile dev --target xtensa-esp32-espidf

.PHONY: build-release  # defer all dependency handling to rust/cargo
build-release:
	cargo +esp build --release --target xtensa-esp32-espidf

.PHONY: run  # defer all dependency handling to rust/cargo
run:
	cargo +esp run --profile dev --target xtensa-esp32-espidf

.PHONY: arp-set-raspberry-dev
arp-set-raspberry-dev:
	sudo arp -s 192.168.87.24 dc:a6:32:85:8b:b5

.PHONY: usb-list
usb-list:
	sudo usbip list -r raspberry-dev

.PHONY: usb-attach
usb-attach:
	sudo usbip attach -r raspberry-dev -b 1-1.4

.PHONY: usb-detach
usb-detach:
	sudo usbip detach -p 00
