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
