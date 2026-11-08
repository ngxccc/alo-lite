.PHONY: all dev dev-desktop run qr build build-desktop mcp test lint fmt package install uninstall clean help

all: dev-desktop

help:
	@echo "AloLite development commands:"
	@echo "  make dev-desktop   - Build frontend & run AloLite Tauri desktop app"
	@echo "  make mcp           - Run AloLite MCP stdio server"
	@echo "  make qr            - Run AloLite in terminal QR login mode"
	@echo "  make build         - Build headless CLI/MCP binary (alolite)"
	@echo "  make build-desktop - Build frontend & Tauri desktop binary (alolite-desktop)"
	@echo "  make test          - Run Rust core/cli tests + desktop TypeScript tests"
	@echo "  make lint          - Check Rust workspace with clippy & TypeScript typecheck"
	@echo "  make fmt           - Format Rust workspace with cargo fmt"
	@echo "  make package       - Build all Linux packages (.deb, .rpm, .AppImage, .tar.gz)"
	@echo "  make install       - Install desktop entry and icon into ~/.local"
	@echo "  make uninstall     - Uninstall desktop entry from ~/.local"
	@echo "  make clean         - Clean build artifacts and dist/"

dev dev-desktop run:
	cd apps/desktop && bun run build
	cargo run -p alolite-desktop --bin alolite-desktop

mcp:
	cargo run -p alolite-cli --bin alolite -- --mcp

qr:
	cargo run -p alolite-cli --bin alolite -- --qr

build:
	cargo build --release -p alolite-cli --bin alolite

build-desktop:
	cd apps/desktop && bun run build
	cargo build --release -p alolite-desktop --bin alolite-desktop
test:
	cargo test --locked
	cd apps/desktop && bun test

lint:
	cargo clippy --workspace --all-targets -- -D warnings
	cd apps/desktop && bun run check

fmt:
	cargo fmt --all
	cd apps/desktop && bun run format
package:
	./packaging/linux/package-all.sh

install:
	cargo run --bin alolite -- --install-desktop

uninstall:
	cargo run --bin alolite -- --uninstall-desktop

clean:
	cargo clean
	rm -rf dist/ apps/desktop/dist/
