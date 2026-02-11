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
- Label (auto-generated with random suffix)
- API key (hidden input)

### config list

List all configured providers.

```bash
ply config list
```

Output includes:

- Label (marked with "(default)" if applicable)
- Unique ID
- Provider type
- Creation timestamp

### config edit

Edit an existing provider configuration.

```bash
ply config edit <label|id>
```

### config delete

Delete a provider configuration.

```bash
ply config delete <label|id>
```

### default

View or set the default provider.

```bash
# View current default
ply default

# Set default by label
ply default openai

# Set default by ID
ply default 123e4567-e89b-12d3-a456-426614174000
```

## Running pi

### Default Provider

Run pi with the configured default provider:

```bash
ply
```

This will:

1. Retrieve the default provider from storage
2. Decrypt the API key using the master key
3. Set the appropriate environment variable
4. Execute `pi --models "<provider>/*"`

### Specific Provider

Run pi with a specific provider without changing the default:

```bash
ply anthropic
ply groq
ply 123e4567-e89b-12d3-a456-426614174000
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

# Set default
ply default anthropic

# Run with default
ply

# Switch to another provider temporarily
ply groq -- --help

# Set as new default
ply default groq
```

### Multiple Provider Management

```bash
# List all providers
ply config list

# Configure OpenAI
ply setup
> Select: openai
> Label: openai-work
> API key: sk-...

# Configure Anthropic
ply setup
> Select: anthropic
> Label: anthropic-personal
> API key: sk-ant-...

# Use specific provider
ply anthropic-personal
```
