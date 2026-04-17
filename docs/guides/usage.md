# Usage Guide

Complete command reference for pyx.

## Commands

### init

Initialize pyx (create data directory and master key).

```bash
pyx init
```

### add

Add a new provider credential.

```bash
pyx add                                    # Interactive: select provider + enter key
pyx add --provider openai                  # Semi-interactive: prompt for key only
pyx add --provider openai --key sk-xxx     # Non-interactive
```

### edit

Edit an existing provider credential.

```bash
pyx edit                                    # Interactive: select provider + enter new key
pyx edit --provider openai                  # Semi-interactive: prompt for new key only
pyx edit --provider openai --key sk-xxx     # Non-interactive
```

### setup (deprecated)

The `setup` command is deprecated. Use `init` followed by `add` instead.

### list

List configured providers.

```bash
pyx list
pyx list --json
```

### delete

Remove a provider.

```bash
pyx delete <provider>
pyx delete              # Interactive selection
pyx delete -y           # Skip confirmation
```

### models

List supported AI models.

```bash
pyx models
pyx models --provider openai
pyx models --json
pyx models --refresh
pyx models update
```

### pi

Manage pi installation.

```bash
pyx pi              # Show pi status
pyx pi install      # Install pi
pyx pi install --force
```

### reset

Reset all pyx data (keys, providers, cache).

```bash
pyx reset
pyx reset -y            # Skip confirmation
```

Requires confirmation. Cannot be undone.

### completion

Generate shell completion.

```bash
pyx completion bash
pyx completion zsh
pyx completion fish
pyx completion powershell
pyx completion bash --install
```

### version

Print version info.

```bash
pyx version
pyx version --json
```

With `--json`, outputs version information as JSON including git metadata.

## Running pi

```bash
pyx                  # All configured providers
pyx openai           # Specific provider
pyx openai -s <id>   # With session
pyx -- --help        # Pass args to pi
```

## Environment Variables

See [Providers Reference](../reference/providers.md) for full list.

| Provider     | Variable               |
| ------------ | ---------------------- |
| anthropic    | `ANTHROPIC_API_KEY`    |
| openai       | `OPENAI_API_KEY`       |
| google       | `GEMINI_API_KEY`       |
| groq         | `GROQ_API_KEY`         |
| azure-openai | `AZURE_OPENAI_API_KEY` |
| xai          | `XAI_API_KEY`          |
| mistral      | `MISTRAL_API_KEY`      |
| minimax      | `MINIMAX_API_KEY`      |

## Custom Providers

See [Providers Reference](../reference/providers.md#custom-providers) for configuration.

## Configuration Files

See [Storage Reference](../reference/storage.md) for file details.
