# AGENTS.md

## What

Rust CLI tool for securely managing AI provider API keys for the pi coding agent.

**Tech Stack**: Rust 1.75+, clap, inquire (interactive prompts), age encryption

**Key Directories**:

- `src/main.rs` - Application entry point
- `src/commands/` - CLI command implementations
- `src/crypto/` - Encryption (age)
- `src/storage/` - Encrypted credential storage
- `src/keys/` - Master key management (OS keyring)
- `src/prompt.rs` - Interactive prompts
- `docs/` - Architecture, contributing, usage

**Data**: `~/.local/share/pyx/` (master.key, database.json - 0600 permissions)

## Why

Provides secure encrypted storage for AI provider credentials and seamless
integration with pi by setting appropriate environment variables.

## How

**Build/Install**:

- `just build` - Build (debug)
- `just build-prod` - Production build (release mode)
- `just install` - Install to `$HOME/.cargo/bin/pyx`

**Test**:

- `just test` - Run unit tests
- `just test-integration` - Run integration tests
- `just test-all` - Run all tests
- `just test-run TestName` - Specific test

**Lint/Format**:

- `just fmt` - Format code
- `just lint` - Run clippy
- `just check` - All checks (fmt, lint, test-all)

**Maintenance**:

- `just clean` - Remove build artifacts
- `just hooks` - Install git pre-commit hooks

## Rules

- **Always use existing justfile targets** - Never run `cargo` commands directly when a `just` target exists
- **Always use `just check` before committing** - Runs fmt, lint, and all tests in one command
- **Fix all lint errors** - Never use suppressions or ignore directives to bypass warnings
- Keep CLI entrypoints thin - parse arguments in command layers and move business logic into testable modules
- Prefer `Result`-based error handling with clear user-facing messages - avoid `unwrap`/`expect` outside tests and unrecoverable startup invariants
- Write machine-readable command output to `stdout`; send diagnostics, prompts, progress, and errors to `stderr`
- Preserve scriptability - any command that can change state or is expected to run in CI/scripts must support a non-interactive path via flags, arguments, or environment variables
- Use `Path`/`PathBuf` and filesystem APIs instead of manual path string building for cross-platform safety
- Minimize dependencies and startup work - favor small, well-maintained crates and avoid unnecessary global state
- Never log, print, or persist secrets outside the encrypted storage flow
- Follow the project's established patterns and conventions

## Docs

Read these for details:

- `docs/architecture.md` - System design, security model, components
- `docs/contributing.md` - Development setup, code standards, adding providers
- `docs/getting-started.md` - Installation and initial configuration
- `docs/usage.md` - Complete command reference
