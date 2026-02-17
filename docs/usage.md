# Usage Guide

Complete reference for all ply commands and options.

## Commands

### setup

Initialize ply configuration and add a new provider.

```bash
ply setup
```

Prompts for:

- Provider selection (number or name)
- API key (hidden input)

If the provider already exists, you'll be asked to confirm override.

### config list

List all configured providers.

```bash
ply config list
```

Output includes:

- Provider name
- Creation timestamp

### config edit

Edit a provider's API key.

```bash
ply config edit <provider>
```

### config delete

Delete a provider configuration.

```bash
ply config delete <provider>
```

## Running pi

### All Configured Providers

Run pi with all configured providers:

```bash
ply
```

This will:

1. Load all configured providers from storage
2. Decrypt each API key using the master key
3. Set the appropriate environment variables
4. Execute `pi --models "provider1/*,provider2/*,..."`

If providers share the same environment variable (e.g., `openai` and `openai-codex` both use `OPENAI_API_KEY`), they must have the same API key or an error will occur.

### Specific Provider

Run pi with a specific provider:

```bash
ply anthropic
ply groq
```

### Passing Arguments to pi

Use `--` to pass arguments directly to pi:

```bash
# Get pi help
ply -- --help

# Use specific model
ply anthropic -- --model claude-sonnet-4

# Disable model filtering
ply -- --model gpt-4o
```

## Shell Completion

Generate and install shell completion scripts.

### Bash

```bash
# One-time source
source <(ply completion bash)

# Persistent installation
ply completion bash > /etc/bash_completion.d/ply  # Linux
ply completion bash > /usr/local/etc/bash_completion.d/ply  # macOS
```

### Zsh

```bash
# Ensure completions are enabled
echo "autoload -U compinit; compinit" >> ~/.zshrc

# Install completion
ply completion zsh > "${fpath[1]}/_ply"
```

### Fish

```bash
# One-time source
ply completion fish | source

# Persistent installation
ply completion fish > ~/.config/fish/completions/ply.fish
```

### PowerShell

```powershell
# One-time source
ply completion powershell | Out-String | Invoke-Expression

# Persistent installation
ply completion powershell > ply.ps1
Add-Content -Path $PROFILE -Value '. .\ply.ps1'
```

## Version

Print version information:

```bash
ply version
```

## Examples

### Complete Workflow

```bash
# Initial setup
ply setup

# Add another provider
ply setup

# List providers
ply config list

# Run with all configured providers
ply

# Run with specific provider
ply anthropic

# Delete a provider
ply config delete openai
```

### Multiple Provider Setup

```bash
# Configure OpenAI
ply setup
> Select: openai
> API key: sk-...

# Configure Anthropic
ply setup
> Select: anthropic
> API key: sk-ant-...

# Configure Groq
ply setup
> Select: groq
> API key: gsk_...

# Run with all providers
ply
```
