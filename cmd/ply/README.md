# ply

A CLI tool for managing AI provider configurations for the pi coding agent.

## Description

Ply provides secure storage and management of API keys for AI providers used by the pi coding agent. It encrypts credentials using AES-256-GCM and automatically configures environment variables when running pi.

## Installation

### From Source

```bash
git clone https://github.com/dkmnx/ply.git
cd ply
make build
./bin/ply --help
```

### Via go install

```bash
go install github.com/dkmnx/ply/cmd/ply@latest
```

### From Release

Download the latest release from [GitHub Releases](https://github.com/dkmnx/ply/releases).

## Quick Start

```bash
# Initialize configuration
ply setup

# List providers
ply config list

# Set default provider
ply default openai

# Run pi
ply
```

## Commands

### setup

Initialize ply and add a new provider:

```bash
ply setup
```

### config

Manage provider configurations:

```bash
ply config list              # List all providers
ply config edit <name|id>    # Edit a provider
ply config delete <name|id>  # Delete a provider
```

### default

View or set the default provider:

```bash
ply default                  # View current default
ply default anthropic        # Set by label
ply default 123abc...         # Set by ID
```

### completion

Generate shell completion:

```bash
ply completion bash        # Bash
ply completion zsh        # Zsh
ply completion fish       # Fish
ply completion powershell # PowerShell
```

### Shell Completion Installation

**Bash:**

```bash
# Source for current session
source <(ply completion bash)

# Persistent (Linux)
sudo ply completion bash > /etc/bash_completion.d/ply

# Persistent (macOS)
ply completion bash > /usr/local/etc/bash_completion.d/ply
```

**Zsh:**

```bash
ply completion zsh > "${fpath[1]}/_ply"
```

**Fish:**

```bash
ply completion fish > ~/.config/fish/completions/ply.fish
```

**PowerShell:**

```powershell
ply completion powershell > ply.ps1
Add-Content -Path $PROFILE -Value '. .\ply.ps1'
```

## Running pi

### Default Provider

Run pi with the configured default:

```bash
ply
```

### Specific Provider

Run with a specific provider:

```bash
ply anthropic
ply openai
ply groq
```

### Passing Arguments to pi

Use `--` to pass arguments directly to pi:

```bash
ply -- --help           # pi help
ply anthropic -- --model claude-sonnet-4
ply -- --model gpt-4o
```

## Building

### Development Build

```bash
go build ./cmd/ply
```

### Production Build

```bash
make build-prod

# Or manually with version info
VERSION=v1.0.0
COMMIT=$(git rev-parse --short HEAD)
DATE=$(date -u +%Y-%m-%dT%H:%M:%SZ)

go build -ldflags="-s -w" \
  -X "main.version=$VERSION" \
  -X "main.commit=$COMMIT" \
  -X "main.date=$DATE" \
  ./cmd/ply
```

## Supported Providers

| Provider | Environment Variable |
|----------|---------------------|
| Anthropic | `ANTHROPIC_API_KEY` |
| OpenAI | `OPENAI_API_KEY` |
| Google Gemini | `GEMINI_API_KEY` |
| Groq | `GROQ_API_KEY` |
| Azure OpenAI | `AZURE_OPENAI_API_KEY` |
| xAI | `XAI_API_KEY` |
| OpenRouter | `OPENROUTER_API_KEY` |
| Vercel AI Gateway | `AI_GATEWAY_API_KEY` |
| ZAI | `ZAI_API_KEY` |
| Mistral | `MISTRAL_API_KEY` |
| MiniMax | `MINIMAX_API_KEY` |
| Hugging Face | `HF_TOKEN` |
| OpenCode | `OPENCODE_API_KEY` |
| Kimi | `KIMI_API_KEY` |

## Files

Configuration is stored in `~/.local/share/ply/`:

| File | Purpose |
|------|---------|
| `master.key` | AES-256-GCM encryption key |
| `database.json` | Encrypted provider credentials |
| `default.txt` | Default provider ID |

## Security

- AES-256-GCM encryption for all API keys
- File permissions: 0600 for all configuration files
- API keys never logged or displayed in plain text

## Documentation

- [Getting Started](../docs/getting-started.md) - Initial setup
- [Usage Guide](../docs/usage.md) - Complete command reference
- [Architecture](../docs/architecture.md) - System design
- [Contributing](../docs/contributing.md) - Development guide
- [Troubleshooting](../docs/troubleshooting.md) - Issue resolution

## License

MIT
