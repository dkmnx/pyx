# pyx

A CLI tool for securely managing AI provider API keys for the pi coding agent.

[![Rust](https://img.shields.io/badge/Rust-1.75+-orange?style=flat&logo=rust)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![CI](https://github.com/dkmnx/pyx/actions/workflows/ci.yml/badge.svg)](https://github.com/dkmnx/pyx/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/dkmnx/pyx)](https://github.com/dkmnx/pyx/releases/latest)

## Overview

Pyx provides secure storage and management of API keys for AI providers. It encrypts credentials using age encryption and integrates with pi by setting the appropriate environment variables.

## Quick Start

```bash
# Install
just install

# Initialize configuration and add a provider
pyx setup

# List providers
pyx list

# Run pi with all configured providers
pyx
```

## Commands

| Command                                           | Description                                 |
| ------------------------------------------------- | ------------------------------------------- |
| `pyx setup`                                       | Initialize and configure pyx with providers |
| `pyx list`                                        | List all configured providers               |
| `pyx delete <provider>`                           | Delete a provider                           |
| `pyx models`                                      | List supported AI models                    |
| `pyx models update`                               | Update models from remote                   |
| `pyx pi install`                                  | Install pi coding agent                     |
| `pyx reset`                                       | Reset pyx configuration                     |
| `pyx <provider>`                                  | Run pi with a specific provider             |
| `pyx`                                             | Run pi with all configured providers        |
| `pyx completion [bash\| zsh\| fish\| powershell]` | Generate shell completion                   |
| `pyx version`                                     | Print version information                   |

## Features

- **Secure Storage**: Age encryption for all API keys
- **Multiple Providers**: Support for Anthropic, OpenAI, Google, Groq, and 50+ more
- **Multi-Provider**: Run pi with all configured providers simultaneously
- **Shell Completion**: Full bash, zsh, fish, and PowerShell support

## Architecture

```text
src/
├── main.rs              # CLI entry point
├── lib.rs               # Library exports
├── commands/            # Command implementations
├── storage/             # Data persistence
├── keys/                # Master key management
├── crypto/              # Age encryption
├── providers/           # Provider handling
├── models/              # Model fetching/caching
└── pi/                  # Pi integration
```

### Data Files

Stored in `~/.local/share/pyx/`:

- `master.key` - Encrypted master key (0600 permissions)
- `database.json` - Encrypted provider API keys
- `models.json` - Cached model list
- `providers.json` - Custom provider-env mappings

### Security Model

1. **Master Key**: 32-byte random key generated during setup
2. **Encryption**: Age encryption (AES-256-GCM)
3. **Passphrase Storage**: OS keyring (SecretService/Keychain/Credential Manager)
4. **Fallback**: `PYX_PASSPHRASE` env var, then "default" (legacy)
5. **File Permissions**: 0600 for sensitive files

### Provider Resolution

Environment variables are resolved with this precedence:

1. `providers.json` - Explicit mappings
2. `settings.json` - Legacy customProviderEnvVars
3. Built-in mappings (50+ providers)
4. Derived naming (`provider` → `PROVIDER_API_KEY`)

## Development

```bash
# Build
just build-prod

# Test
just test

# Format & lint
just fmt
just lint

# Install
just install
```

## Documentation

- [Getting Started](docs/getting-started.md) - Installation and setup
- [Usage](docs/usage.md) - Command reference
- [Architecture](docs/architecture.md) - System design and security
- [Contributing](docs/contributing.md) - Development guide
- [Troubleshooting](docs/troubleshooting.md) - Common issues

## License

[MIT](LICENSE)
