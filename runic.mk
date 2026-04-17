# Sample runic.mk for the runic project itself.
# Descriptions use the `## text` convention.

## Build in release mode
build:
	cargo build --release

## Run the test suite
test:
	cargo test

## Format, clippy, check
check:
	cargo fmt --check
	cargo clippy --all-targets -- -D warnings
	cargo check

## Install runic to ~/.cargo/bin
install:
	cargo install --path .

## Deploy to $(HOST) as $(USER)
deploy:
	@echo "deploying to $(USER)@$(HOST)"

.PHONY: build test check install deploy
