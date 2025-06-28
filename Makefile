# ftdv Makefile

.PHONY: all build install clean test lint format run-debug help

# Build targets
all: build

build:
	cargo build --release

build-dev:
	cargo build

install: build
	cargo install --path .

# Development targets
test:
	cargo test

lint:
	cargo clippy -- -D warnings

format:
	cargo fmt

clean:
	cargo clean

# Debug and utilities
run-debug:
	FTDV_DEBUG=1 cargo run

check:
	cargo check

# Installation and setup
install-all: install
	@echo "Installation complete!"
	@echo ""
	@echo "Usage:"
	@echo "  ftdv                               # Working directory changes"
	@echo "  ftdv branch1 branch2               # Compare branches"
	@echo "  ftdv --cached                      # Staged changes"
	@echo "  git diff | ftdv                    # Pipe mode (backward compatibility)"

# Git integration setup
setup-git:
	@echo "Setting up git to use ftdv as pager..."
	git config --global core.pager ftdv
	@echo "Git configured to use ftdv"
	@echo ""
	@echo "Test with: git diff"

uninstall-git:
	@echo "Removing ftdv from git configuration..."
	git config --global --unset core.pager || true
	@echo "Git configuration cleaned up"

# Example config
example-config:
	@echo "Creating example config at ~/.config/ftdv/config.yaml"
	@mkdir -p ~/.config/ftdv
	@cp config.example.yaml ~/.config/ftdv/config.yaml
	@echo "Edit ~/.config/ftdv/config.yaml to customize"

# Help
help:
	@echo "ftdv - A TUI diff pager"
	@echo ""
	@echo "Build targets:"
	@echo "  build             Build release binary"
	@echo "  build-dev         Build debug binary"
	@echo "  install           Install ftdv binary"
	@echo "  install-all       Install binary"
	@echo ""
	@echo "Development:"
	@echo "  test              Run tests"
	@echo "  lint              Run clippy linter"
	@echo "  format            Format code"
	@echo "  check             Check compilation"
	@echo "  clean             Clean build artifacts"
	@echo ""
	@echo "Git integration:"
	@echo "  setup-git         Set up git to use ftdv as pager"
	@echo "  uninstall-git     Remove git integration"
	@echo ""
	@echo "Configuration:"
	@echo "  example-config    Create example config file"