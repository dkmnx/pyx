# AGENTS.md

## What

Go CLI tool for securely managing AI provider API keys for the pi coding agent.

**Tech Stack**: Go 1.24+, Cobra, AES-256-GCM encryption

**Key Directories**:

- `cmd/ply/main.go` - Application entry point
- `internal/cmd/` - CLI command implementations
- `internal/crypto/` - Encryption (AES-256-GCM)
- `internal/database/` - Encrypted credential storage
- `internal/keys/` - Master key management (OS keyring)
- `docs/` - Architecture, contributing, usage

**Data**: `~/.local/share/ply/` (master.key, database.json - 0600 permissions)

## Why

Provides secure encrypted storage for AI provider credentials and seamless
integration with pi by setting appropriate environment variables.

## How

**Build/Install**:

- `make build` - Build to `bin/ply`
- `make install` - Install to `$GOPATH/bin/ply`
- `make run ARGS="..."` - Run directly

**Test**:

- `make test` - Run all tests with race detection
- `make test-v` - Verbose test output
- `make test-run RUN=TestName` - Specific test

**Lint/Format**:

- `make fmt` - Format code
- `make lint` - Run golangci-lint
- `make vet` - Run go vet
- `make check` - All checks (fmt, vet, lint, test)

**Maintenance**:

- `make mod-tidy` - Tidy go modules
- `make clean` - Remove build artifacts

## Docs

Read these for details:

- `docs/architecture.md` - System design, security model, components
- `docs/contributing.md` - Development setup, code standards, adding providers
- `docs/getting-started.md` - Installation and initial configuration
- `docs/usage.md` - Complete command reference
