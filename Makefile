.DEFAULT_GOAL := help

INSTALL_DIR := $(HOME)/Applications
BUILT_APP := target/release/bundle/macos/Desktop Pet.app
INSTALLED_APP := $(INSTALL_DIR)/Desktop Pet.app

.PHONY: help dev preview format test

help:
	@printf '%s\n' \
	  'Usage: make <target>' \
	  '' \
	  '  help    Show available targets (default)' \
	  '  dev     Run the Tauri desktop app in development mode' \
	  '  build   Build the release macOS .app bundle' \
	  '  preview Build and open the local macOS app independently of the terminal' \
	  '  deploy  Build, replace and launch the app in ~/Applications' \
	  '  format  Format Rust and frontend files' \
	  '  test    Run frontend and Rust workspace tests'

dev:
	pnpm tauri dev

.PHONY: build
build: ## Build release .app bundle
	pnpm tauri build

preview: build
	open "$(BUILT_APP)"

.PHONY: deploy
deploy: build ## Build and replace installed app at ~/Applications
	@echo "→ removing old $(INSTALLED_APP)..."
	rm -rf "$(INSTALLED_APP)"
	@echo "→ installing new build..."
	mkdir -p "$(INSTALL_DIR)"
	cp -R "$(BUILT_APP)" "$(INSTALLED_APP)"
	@echo "→ launching..."
	open "$(INSTALLED_APP)"
	@echo "✓ deployed to $(INSTALLED_APP)"

format:
	cargo fmt --all
	pnpm exec prettier --write . --ignore-path .gitignore

test:
	pnpm test
	cargo test --workspace
