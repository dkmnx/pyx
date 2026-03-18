# Cross-platform justfile for pyx
# Works on Windows, macOS, and Linux

# Set shell for Windows (PowerShell) only
set windows-shell := ["pwsh", "-NoProfile", "-Command"]

# Fast scrypt work factor for tests (production uses default 18)
export PYX_SCRYPT_WORK_FACTOR := "14"

# Variables
app_name := "pyx"
build_dir := "./target/release"

# Version info - cross-platform
version := `git describe --tags --always 2>&1`
commit := `git rev-parse --short HEAD 2>&1`
date := `git log -1 --format=%aI 2>&1`

# Build the application
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
    @echo "Available targets:"
    @echo "  build             - Build the application (debug)"
    @echo "  build-prod        - Build for production (release)"
    @echo "  test              - Run unit tests"
    @echo "  test-v            - Run tests with verbose output"
    @echo "  test-run RUN      - Run specific test"
    @echo "  test-integration  - Run integration tests"
    @echo "  test-all          - Run all tests (unit + integration)"
    @echo "  fmt               - Format code"
    @echo "  fmt-check         - Check formatting (non-mutating, for CI)"
    @echo "  lint              - Lint code (clippy)"
    @echo "  clean             - Clean build artifacts"
    @echo "  install           - Install binary"
    @echo "  check             - Run all checks (fmt, lint, test-all)"
    @echo "  check-ci          - CI checks (fmt-check, lint, test-all)"
    @echo "  help              - Show this help message"
    @echo ""
    @echo "Examples:"
    @echo "  just build"
    @echo "  just test"
    @echo "  just test-run test_get_env_var"
