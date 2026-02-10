# ply

A CLI tool for managing AI provider configurations.

## Installation

```bash
go install github.com/dkmnx/ply/cmd/ply@latest
```

## Usage

### Initialize Configuration

```bash
ply setup
```

### Manage Providers

```bash
# List all configured providers
ply config list

# Edit a provider configuration
ply config edit openai

# Delete a provider configuration
ply config delete openai
```

### Set Default Provider

```bash
# View current default provider
ply default

# Set a provider as default
ply default openai
```

### Shell Completion

```bash
# Bash
source <(ply completion bash)

# Zsh
ply completion zsh > "${fpath[1]}/_ply"

# Fish
ply completion fish | source

# PowerShell
ply completion powershell | Out-String | Invoke-Expression
```

### Version

```bash
ply version
```

## Building

```bash
# Build for development
go build ./cmd/ply

# Build for production
go build -ldflags="-s -w" -X 'main.version=v1.0.0' -X 'main.commit=$(git rev-parse --short HEAD)' -X 'main.date=$(date -u +%Y-%m-%dT%H:%M:%SZ)' ./cmd/ply
```
