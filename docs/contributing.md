# Contributing

Development guidelines for pyx.

## Setup

```bash
git clone https://github.com/dkmnx/pyx.git
cd pyx
cargo build
```

## Commands

```bash
cargo build              # Debug build
cargo build --release    # Release build
cargo test --lib         # Run tests
cargo fmt                # Format code
cargo clippy             # Lint
```

## Code Style

- Run `cargo fmt` before committing
- Address all clippy warnings
- Document public functions

## Project Structure

```text
src/
├── commands/     # CLI commands
├── storage/      # Data persistence
├── keys/         # Key management
├── crypto/       # Encryption
├── providers/    # Provider handling
├── models/       # Model fetching
└── pi/           # Pi integration
```

## Adding a Provider

1. Add mapping to `src/providers/mapping.rs`
2. Update tests in `src/providers/mapping.rs`

## Pull Requests

1. Ensure tests pass: `cargo test --lib`
2. Format code: `cargo fmt`
3. Fix warnings: `cargo clippy`
4. Create PR with descriptive message
