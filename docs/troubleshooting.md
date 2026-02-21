# Troubleshooting

Common issues and their solutions.

## Installation

### "command not found: ply"

Ensure ply is in your PATH:

```bash
# Add to PATH (add to ~/.bashrc or ~/.zshrc)
export PATH="$HOME/go/bin:$PATH"

# Verify installation
which ply
ply version
```

### Permission Denied

If you get permission errors during installation:

```bash
# Install to user-local bin
mkdir -p ~/bin
export PATH="$HOME/bin:$PATH"
go install github.com/dkmnx/ply/cmd/ply@latest
```

## Configuration

### "No providers configured"

Run `ply setup` to initialize configuration and add a provider:

```bash
ply setup
```

### "Error loading master key"

The encryption key is missing or corrupted:

```bash
# This will regenerate the key but existing credentials will be lost
ply setup
```

### "Error decrypting API key"

The master key and database are out of sync. This happens if:

- The database was copied from another machine
- The master key was regenerated
- Files were manually modified

**Recovery is not possible** - the encrypted data is permanently lost.

## Provider Issues

### "Provider not found"

The specified provider label or ID doesn't exist:

```bash
# List all providers
ply config list
```

### "Unsupported provider"

The provider name is not recognized:

```bash
# Check supported providers during setup
ply setup
> Select a provider:
>   1. anthropic
>   2. openai
>   ...
```

### "API key cannot be empty"

The API key prompt requires input. Enter your actual API key.

### Invalid API key format

Some providers require specific key formats:

- **Anthropic**: Starts with `sk-ant-...`
- **OpenAI**: Starts with `sk-...`
- **Google**: API key format varies by service

## Shell Completion

### Completion not working in Bash

```bash
# Verify completion is loaded
type _ply_completion

# Re-source completion
source <(ply completion bash)
```

### Completion not working in Zsh

```bash
# Ensure compinit is loaded
autoload -U compinit
compinit

# Verify completion file exists
ls ${fpath[1]}
```

### Completion not working in Fish

```bash
# Verify completion directory exists
ls ~/.config/fish/completions/

# Re-source completions
ply completion fish | source
```

## pi Integration

### "Error running pi"

Ensure pi is installed and in PATH:

```bash
which pi
pi --help
```

### Models not filtering correctly

Model filtering uses the provider's model prefix:

```bash
# This filters to Anthropic models only
ply anthropic

# Skip filtering entirely
ply -- --model gpt-4
```

## File Permissions

### Permission issues on Linux

```bash
# Fix ply data directory permissions
chmod 700 ~/.local/share/ply
chmod 600 ~/.local/share/ply/*
```

### Permission issues on macOS

```bash
# Check and fix permissions
ls -la ~/.local/share/ply/
chmod 600 ~/.local/share/ply/*
```

## Debug Mode

Enable verbose output by checking command execution:

```bash
# Enable shell tracing
set -x
ply
set +x
```

## Log Files

Ply doesn't create log files by default. For debugging:

```bash
# Check ply version
ply version

# Check installed location
which ply

# Verify database contents (decrypted)
# Not possible - data is encrypted
```

## Environment Variables

### Check set variables

```bash
# After running ply, check current env
env | grep -E 'ANTHROPIC|OPENAI|GEMINI|GROQ'
```

### Variables overwritten

Other processes may set the same variables:

```bash
# Use -- to ensure ply sets variables last
ply -- pi --help
```

## Resetting Everything

To reset ply configuration (destroys all stored credentials):

```bash
# Backup if needed
cp ~/.local/share/ply/database.json ~/ply-backup.json

# Reset
rm -rf ~/.local/share/ply
ply setup
```

## Getting Help

- Check this guide for your issue
- Review command help: `ply --help`
- Review subcommand help: `ply config --help`
