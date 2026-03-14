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
just test               # Run tests
just fmt                # Format code
just lint               # Lint
just check              # Run all checks (fmt, lint, test-all)
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
