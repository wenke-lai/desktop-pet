.DEFAULT_GOAL := help

.PHONY: help dev build preview format test

help:
	@printf '%s\n' \
	  'Usage: make <target>' \
	  '' \
	  '  help    Show available targets (default)' \
	  '  dev     Run the Tauri desktop app in development mode' \
	  '  build   Build the desktop app locally without installer packaging' \
	  '  preview Build and open the local macOS app independently of the terminal' \
	  '  format  Format Rust and frontend files' \
	  '  test    Run frontend and Rust workspace tests'

dev:
	pnpm tauri dev

build:
	pnpm tauri build --no-bundle

preview:
	pnpm tauri build --bundles app
	open "target/release/bundle/macos/Desktop Pet.app"

format:
	cargo fmt --all
	pnpm exec prettier --write . --ignore-path .gitignore

test:
	pnpm test
	cargo test --workspace
