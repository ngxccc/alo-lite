.PHONY: all dev dev-desktop run qr build build-desktop mcp test lint fmt docs package install uninstall clean version-check version-bump release help

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
	@echo "  make fmt           - Format Rust workspace, Desktop UI, scripts & Markdown docs"
	@echo "  make docs          - Build and open alolite-core private Rustdoc in browser"
	@echo "  make package       - Build all Linux packages (.deb, .rpm, .AppImage, .tar.gz)"
	@echo "  make install       - Install desktop entry and icon into ~/.local"
	@echo "  make uninstall     - Uninstall desktop entry from ~/.local"
	@echo "  make version-check - Verify all workspace manifests agree on version"
	@echo "  make version-bump  - Bump version across manifests (e.g. make version-bump VERSION=0.1.1)"
	@echo "  make release       - Cut release: bump, changelog, tag (e.g. make release VERSION=minor)"
	@echo "  make clean         - Clean build artifacts and dist/"

dev-desktop:
	cd apps/desktop && bun run build
	cargo run -p alolite-desktop --bin alolite-desktop -- --dev

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
	bun run format:check

fmt:
	cargo fmt --all
	cd apps/desktop && bun run format
	bun run format
docs:
	cargo doc -p alolite-core --no-deps --document-private-items --open

package:
	./packaging/linux/package-all.sh

install:
	cargo run --bin alolite -- --install-desktop

uninstall:
	cargo run --bin alolite -- --uninstall-desktop

clean:
	cargo clean
	rm -rf dist/ apps/desktop/dist/

version-check:
	bun scripts/version.ts check

version-bump:
	bun scripts/version.ts bump $(VERSION)

release:
	bun scripts/release.ts $(VERSION)
