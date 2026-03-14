# Contributing

Development guidelines for pyx.

## Setup

```bash
git clone https://github.com/dkmnx/pyx.git
cd pyx
just build
```

## Commands

```bash
just build              # Debug build
just build-prod         # Release build
just test               # Run unit tests
just test-all           # Run all tests (unit + integration)
just fmt                # Format code
just fmt-check          # Check formatting (non-mutating)
just lint               # Lint
just check              # Run all checks (fmt, lint, test-all)
just check-ci           # CI checks (fmt-check, lint, test-all)
```

## Code Style

- Run `just fmt` before committing
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

1. Ensure tests pass: `just test`
2. Format code: `just fmt`
3. Fix warnings: `just lint`
4. Run all checks: `just check`
5. Create PR with descriptive message

## CI/CD

This project uses GitHub Actions for continuous integration and releases:

- **CI**: Runs on every push and PR (Linux full tests, macOS/Windows build + smoke)
- **Release**: Runs when a version tag is pushed (e.g., `v0.1.0`)

### Running CI checks locally

To match CI behavior:

```bash
just check-ci  # Non-mutating checks (fmt-check, lint, tests)
```

### Creating a Release

1. Update version in `Cargo.toml`
2. Create an annotated tag:

```bash
git tag -a v0.1.0 -m "Release v0.1.0"
git push origin v0.1.0
```

The release workflow will build binaries for Linux, macOS, and Windows, then create a GitHub Release.
