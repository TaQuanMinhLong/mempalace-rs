# MemPalace Justfile

# Default recipe - show help
default:
    @just --list

# Build release binary
build:
    cargo build --release

# Build debug binary
build-dev:
    cargo build

# Run tests
test:
    cargo test

# Run Criterion benchmarks
bench-criterion:
    cargo bench

bench-cmd:
    cargo run -r --features bench -- benchmark

bench: bench-criterion bench-cmd

# Run tests with output
test-verbose:
    cargo test -- --nocapture

# Run clippy lints
lint:
    cargo clippy --all-targets --all-features -- -D warnings

# Format code
fmt:
    cargo fmt --all

# Clean build artifacts
clean:
    cargo clean

# Install mempalace to ~/.local/bin
install: build
    mkdir -p ~/.local/bin
    cp target/release/mempalace ~/.local/bin/mempalace
    chmod +x ~/.local/bin/mempalace
    @echo "Installed to ~/.local/bin/mempalace"

# Uninstall from ~/.local/bin
uninstall:
    rm -f ~/.local/bin/mempalace

# Initialize mempalace config
init path:
    ~/.local/bin/mempalace init {{path}}

# Initialize in home directory
init-home:
    ~/.local/bin/mempalace init ~/.mempalace

# Mine a directory (default: current directory)
mine path=".":
    ~/.local/bin/mempalace mine {{path}}

# Mine current project
mine-here: build install
    just mine .

# Search mempalace
search query:
    ~/.local/bin/mempalace search "{{query}}"

# Check status
status:
    ~/.local/bin/mempalace status

# Wake-up (optionally with wing)
wake-up wing="":
    ~/.local/bin/mempalace wake-up {{if wing != "" { "--wing " + wing} else {""}}}

# Compress a wing
compress wing:
    ~/.local/bin/mempalace compress --wing {{wing}}

# Repair index
repair:
    ~/.local/bin/mempalace repair

# Split files
split path:
    ~/.local/bin/mempalace split {{path}}

# Serve MCP server
serve:
    ~/.local/bin/mempalace serve

# Build and run directly from target
run *args:
    ./target/release/mempalace {{args}}

# Run debug build
run-dev *args:
    ./target/debug/mempalace {{args}}

# Watch for changes and rebuild (requires cargo-watch)
watch:
    cargo watch -x build

# Full CI check
ci: fmt lint test

# Quick start - build, install, init
setup: build install init-home
    @echo ""
    @echo "Run 'just mine-here' to mine your first project!"
