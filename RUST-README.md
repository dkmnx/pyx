# Pyx Rust Implementation

Rust rewrite of the pyx CLI tool for securely managing AI provider API keys.

## Status

**Phase**: Implementation Complete (Phases 0-6)
**Tests**: 37 passing, 4 ignored
**Build**: ✅ Compiles successfully

## Quick Start

### Build

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Using build script
./scripts/build.sh
```

### Run Tests

```bash
cargo test --lib

# Using test script
./scripts/test.sh
```

### Install

```bash
cargo install --path .
```

## Available Commands

```bash
# Initialize pyx
pyx setup

# List configured providers
pyx list
pyx list --json

# Add a provider (prompts for API key)
pyx <provider-name>

# Run with specific provider
pyx openai
pyx anthropic

# Run with session
pyx -s <session-uuid>
pyx openai -s <session-uuid>

# Manage models
pyx models list
pyx models update

# Remove provider
pyx delete <provider-name>

# Generate completions
pyx completion bash >> ~/.bashrc
pyx completion zsh >> ~/.zshrc

# Install pi coding agent
pyx pi-install

# Reset everything
pyx reset

# Version info
pyx version
pyx version --json
```

## Architecture

### Directory Structure

```
src-rust/
├── main.rs              # CLI entry point
├── lib.rs               # Library exports
├── cli.rs               # Clap CLI definitions
├── error.rs             # Error types
├── commands/            # Command implementations
│   ├── setup.rs        # Interactive setup
│   ├── list.rs         # List providers
│   ├── delete.rs       # Delete provider
│   ├── models.rs       # Model management
│   ├── version.rs      # Version info
│   ├── completion.rs   # Shell completions
│   ├── reset.rs        # Reset all data
│   └── root.rs         # Root execution
├── storage/            # Data persistence
│   ├── paths.rs        # Directory resolution
│   ├── database.rs     # Provider database
│   ├── settings.rs     # User settings
│   ├── models_cache.rs # Model caching
│   ├── providers_env.rs# Provider-env mappings
│   └── atomic_write.rs # Safe file writes
├── keys/               # Key management
│   ├── manager.rs      # Master key lifecycle
│   └── keyring.rs      # OS keyring integration
├── crypto/             # Cryptography
│   └── legacy_age.rs   # Age encryption
├── providers/          # Provider handling
│   ├── mapping.rs      # Env-var mapping
│   └── validation.rs   # Name validation
├── models/             # Model handling
│   ├── fetch.rs        # Remote fetching
│   └── parse.rs        # Response parsing
└── pi/                 # Pi integration
    └── exec.rs         # Process execution
```

### Data Files

Stored in `~/.local/share/ply/` (or `$XDG_DATA_HOME/ply`):

- `master.key` - Encrypted master key (0600 permissions)
- `database.json` - Encrypted provider API keys
- `models.json` - Cached model list
- `settings.json` - User settings
- `providers.json` - Provider-env mappings (new)

### Security Model

1. **Master Key**: 32-byte random key generated during setup
2. **Encryption**: AES-256-GCM via age library
3. **Passphrase Storage**: OS keyring (SecretService/Keychain/Credential Manager)
4. **Fallback**: `PLY_PASSPHRASE` env var, then "default" (legacy)
5. **File Permissions**: 0600 for sensitive files

## Provider Resolution

Environment variables are resolved with this precedence:

1. `providers.json` - Explicit mappings
2. `settings.json` - Legacy customProviderEnvVars
3. Built-in mappings (50+ providers)
4. Derived naming (`provider` → `PROVIDER_API_KEY`)

### Built-in Providers

Core: openai, anthropic, google, google-vertex, azure, azure-openai

Chinese: minimax, minimax-cn, zhipu, baichuan, moonshot

Other: groq, mistral, cohere, together, anyscale, replicate, perplexity, friendli, vercel-ai, vercel-openai, vercel-anthropic

## Testing

```bash
# Run all tests
cargo test --lib

# Run specific test
cargo test --lib test_list_with_providers

# Run with output
cargo test --lib -- --nocapture

# Coverage (requires cargo-tarpaulin)
cargo tarpaulin --out html
```

### Test Coverage

- **Storage**: All modules tested (paths, database, settings, models_cache, atomic_write)
- **Providers**: Validation and mapping tested
- **Commands**: Version, list, delete, models, completion tested
- **Root**: Provider selection logic tested
- **Integration**: Real database fixtures used

### Ignored Tests

4 tests require interactive/real setup:
- Crypto compatibility (needs Go fixtures)
- Keyring roundtrip (needs actual keyring)
- Key manager generate/load (needs keyring)
- Interactive passphrase prompt (needs TTY)

## Migration from Go

**Important**: The Rust implementation uses a different age encryption format.

### One-Time Migration

A migration tool will be provided to:
1. Read Go-encrypted data
2. Decrypt with Go implementation
3. Re-encrypt with Rust-compatible format

Until then, existing Go users should:
1. Keep Go binary installed
2. Use Go version until migration tool is ready
3. Or re-run `pyx setup` and re-enter API keys

## Development

### Prerequisites

- Rust 1.88.0+
- Cargo
- OS keyring support (libsecret on Linux)

### Build Commands

```bash
# Debug build
cargo build

# Release build
cargo build --release

# Run directly
cargo run -- version
cargo run -- list

# Install to PATH
cargo install --path .
```

### Code Style

```bash
# Format code
cargo fmt

# Lint
cargo clippy

# Check
cargo check
```

## Known Limitations

1. **Crypto Compatibility**: Cannot decrypt Go-generated data (see Migration section)
2. **Models Fetch**: Returns placeholder data (TODO: integrate with pi-mono)
3. **Extension Providers**: providers.json must be manually configured
4. **Migration Tool**: Not yet implemented

## Roadmap

- [ ] Migration tool (Go → Rust format)
- [ ] Pi-mono integration for models fetch
- [ ] Extension provider auto-discovery
- [ ] GitHub Actions CI/CD
- [ ] Release packaging (deb, rpm, Homebrew)
- [ ] Security audit
- [ ] Performance optimization

## License

Same as Go implementation (see LICENSE)

## Contributing

See the main project's CONTRIBUTING.md for guidelines.

## Acknowledgments

Based on the Go implementation by @dkmnx. See the main repository for original authors and contributors.
