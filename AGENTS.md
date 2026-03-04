# AGENTS.md

## What

Go CLI tool for securely managing AI provider API keys for the pi coding agent.

**Tech Stack**: Go 1.24+, Cobra, tap (interactive prompts), AES-256-GCM encryption

**Key Directories**:

- `cmd/ply/main.go` - Application entry point
- `internal/cmd/` - CLI command implementations
- `internal/crypto/` - Encryption (AES-256-GCM)
- `internal/database/` - Encrypted credential storage
- `internal/keys/` - Master key management (OS keyring)
- `internal/prompt/` - Interactive prompts (tap library)
- `docs/` - Architecture, contributing, usage

**Data**: `~/.local/share/ply/` (master.key, database.json - 0600 permissions)

## Why

Provides secure encrypted storage for AI provider credentials and seamless
integration with pi by setting appropriate environment variables.

## How

**Build/Install**:
- `just build` - Build to `bin/ply`
- `just build-prod` - Production build (stripped binary)
- `just install` - Install to `$GOPATH/bin/ply`
- `just run ARGS` - Run directly

**Test**:
- `just test` - Run all tests with race detection
- `just test-v` - Verbose test output
- `just test-run RUN=TestName` - Specific test

**Lint/Format**:
- `just fmt` - Format code
- `just vet` - Run go vet
- `just lint` - Run golangci-lint
- `just check` - All checks (fmt, vet, lint, test)

**Security/Maintenance**:
- `just security` - Run gosec and govulncheck
- `just mod-tidy` - Tidy go modules
- `just clean` - Remove build artifacts
- `just deps` - Install dependencies

## Docs

Read these for details:

- `docs/architecture.md` - System design, security model, components
- `docs/contributing.md` - Development setup, code standards, adding providers
- `docs/getting-started.md` - Installation and initial configuration
- `docs/usage.md` - Complete command reference
