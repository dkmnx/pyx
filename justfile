# Cross-platform justfile for pyx
# Works on Windows, macOS, and Linux

# Set shell for Windows (PowerShell) only
set windows-shell := ["pwsh", "-NoProfile", "-Command"]

# Fast scrypt work factor for tests (production uses default 18)
export PYX_SCRYPT_WORK_FACTOR := "15"

# Variables
app_name := "pyx"
build_dir := "./target/release"

# Version info - cross-platform
version := `git describe --tags --always 2>&1`
commit := `git rev-parse --short HEAD 2>&1`
date := `git log -1 --format=%aI 2>&1`

# Install all dependencies (Rust toolchain + crate dependencies)
deps:
    @echo "Installing dependencies..."
    cargo check
    @echo "Dependencies installed!"
build:
    @echo "Building {{ app_name }}..."
    cargo build
    @echo "Built: target/debug/{{ app_name }}"

# Build for production (release mode)
build-prod:
    @echo "Building {{ app_name }} (production)..."
    cargo build --release
    @echo "Built: target/release/{{ app_name }}"

# Run tests
test:
    @echo "Running tests..."
    cargo test --lib

# Run tests with verbose output
test-v:
    @echo "Running tests (verbose)..."
    cargo test --lib -- --nocapture

# Run specific test
test-run RUN:
    @echo "Running specific test..."
    cargo test --lib -- {{ RUN }}

# Run integration tests
test-integration:
    @echo "Running integration tests..."
    cargo test --test root_parity
    cargo test --test subcommand_tests

# Run all tests (lib + integration)
test-all: test test-integration

# Format code
fmt:
    @echo "Formatting code..."
    cargo fmt

# Check formatting (non-mutating, for CI)
fmt-check:
    @echo "Checking formatting..."
    cargo fmt -- --check

# Lint code
lint:
    @echo "Linting code..."
    cargo clippy -- -D warnings

# Clean build artifacts
clean:
    @echo "Cleaning..."
    cargo clean
    rm -rf dist/

# Uninstall binary
uninstall:
    @echo "Uninstalling {{ app_name }}..."
    cargo uninstall {{ app_name }}
    @echo "Uninstalled {{ app_name }}!"

# Install binary
install:
    @echo "Installing {{ app_name }}..."
    cargo install --path . --force

# All checks before committing (mutates files via fmt)
check: fmt lint test-all
    @echo "All checks passed!"

# CI-friendly checks (non-mutating, for CI pipelines)
check-ci: fmt-check lint test-all
    @echo "All CI checks passed!"

# Extract latest changelog entry to a temp file
release-notes:
    awk '/^## \[/{c++; if(c>1)exit; if(c==1){next}} c>0' CHANGELOG.md | sed '/./,$!d' > /tmp/pyx-release-notes.txt

# Dry-run release to verify goreleaser config and changelog extraction
pre-release: check release-notes
    @echo "Running goreleaser dry-run..."
    goreleaser release --clean --snapshot --skip=validate --release-notes /tmp/pyx-release-notes.txt
    @echo "Dry-run complete!"

# Build release with goreleaser (snapshot, no publish)
goreleaser: release-notes
    @echo "Running goreleaser snapshot..."
    goreleaser release --clean --snapshot --skip=validate --release-notes /tmp/pyx-release-notes.txt
    @echo "Snapshot complete!"

# Build and publish release to GitHub (requires a git tag)
release: check release-notes
    @echo "Publishing release..."
    goreleaser release --clean --release-notes /tmp/pyx-release-notes.txt
    @echo "Release published!"

# Install git hooks
hooks:
    @echo "Installing git hooks..."
    @cp .githooks/pre-commit .git/hooks/pre-commit
    @chmod +x .git/hooks/pre-commit
    @echo "Git hooks installed successfully!"
    @echo "Pre-commit hook runs: fmt --check, lint, tests (if src/ changed)"

# Show help
[default]
help:
    @just --list
