.PHONY: all dev run qr build release test lint fmt package install uninstall clean help

# Default target
all: dev

help:
	@echo "AloLite development commands:"
	@echo "  make dev       - Run AloLite GUI (development mode)"
	@echo "  make run       - Run AloLite GUI"
	@echo "  make qr        - Run AloLite in terminal QR login mode"
	@echo "  make build     - Build release binary"
	@echo "  make test      - Run all unit and integration tests"
	@echo "  make lint      - Check code with cargo clippy"
	@echo "  make fmt       - Format Rust code with cargo fmt"
	@echo "  make package   - Build all Linux packages (.deb, .rpm, .AppImage, .tar.gz)"
	@echo "  make install   - Install desktop app and icon into ~/.local"
	@echo "  make uninstall - Uninstall desktop app from ~/.local"
	@echo "  make clean     - Clean build artifacts and dist/"

dev:
	cargo run --bin alolite

run:
	cargo run --bin alolite

qr:
	cargo run --bin alolite -- --qr

build release:
	cargo build --release --bin alolite

test:
	cargo test --lib
	cargo test --tests

lint:
	cargo clippy --all-targets

fmt:
	cargo fmt --all

package:
	./packaging/linux/package-all.sh

install:
	cargo run --bin alolite -- --install-desktop

uninstall:
	cargo run --bin alolite -- --uninstall-desktop

clean:
	cargo clean
	rm -rf dist/
