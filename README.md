# ply

```text
██████  ██
██  ██  ██
████  ██
██    ██
```

[![Go Version](https://img.shields.io/badge/Go-1.24-00ADD8?style=flat&logo=go)](https://go.dev/dl/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A CLI tool for managing AI provider configurations for the pi coding agent.

## Overview

Ply provides secure storage and management of API keys for AI providers.
It encrypts credentials using AES-256-GCM and integrates with pi by setting
the appropriate environment variables.

## Quick Start

```bash
# Install
go install github.com/dkmnx/ply/cmd/ply@latest

# Initialize master key
ply init

# Add a provider
ply setup

# List providers
ply config list

# Run pi with all configured providers
ply
```

## Commands

| Command | Description |
|---------|-------------|
| `ply init` | Initialize master encryption key |
| `ply setup` | Add a new provider configuration |
| `ply config list` | List all configured providers |
| `ply config edit <provider>` | Edit a provider's API key |
| `ply config delete <provider>` | Delete a provider |
| `ply models` | List supported AI models |
| `ply [provider]` | Run pi with a specific provider |
| `ply` | Run pi with all configured providers |
| `ply completion [bash\|zsh\|fish\|powershell]` | Generate shell completion |
| `ply version` | Print version information |

## Features

- **Secure Storage**: AES-256-GCM encryption for all API keys
- **Multiple Providers**: Support for Anthropic, OpenAI, Google, Groq, and more
- **Multi-Provider**: Run pi with all configured providers simultaneously
- **Model Filtering**: Automatically filters models by provider
- **Shell Completion**: Full bash, zsh, fish, and PowerShell support

## Project Structure

```text
ply/
├── cmd/
│   └── ply/           # Main application entry point
├── internal/
│   ├── cmd/           # CLI commands implementation
│   ├── crypto/        # Encryption utilities (AES-256-GCM)
│   ├── database/      # Encrypted credential storage
│   ├── fs/            # File system utilities
│   ├── keys/          # Master key management
│   ├── models/        # AI models list
│   ├── prompt/        # Interactive input utilities
│   ├── providers/     # Provider utilities
│   └── session/       # Session management
├── docs/              # Documentation
├── Makefile           # Build automation
└── README.md          # This file
```

## Documentation

- [Getting Started](docs/getting-started.md) - Setup and configuration
- [Usage Guide](docs/usage.md) - Complete command reference
- [Architecture](docs/architecture.md) - System design and security
- [Contributing](docs/contributing.md) - Development guide
- [Troubleshooting](docs/troubleshooting.md) - Common issues and solutions

## License

[MIT](LICENSE)
