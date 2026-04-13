# Usage Guide

Complete command reference for pyx.

## Commands

### setup

Initialize pyx and manage providers (add or edit).

```bash
pyx setup
```

This command:

- Initializes pyx on first run (creates master key, stores passphrase)
- Adds new providers
- Edits existing providers (prompts to confirm override)

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
pyx pi install --auto
```

### reset

Delete all pyx data (keys, providers, cache).

```bash
pyx reset
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
