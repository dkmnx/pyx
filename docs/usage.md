# Usage Guide

Complete reference for all ply commands and options.

## Commands

### setup

Initialize ply configuration and add a new provider.

```bash
ply setup
```

This command:
- Creates the data directory if it doesn't exist
- Creates a new 32-byte encryption key if one doesn't exist
- Stores the key securely in OS keyring (or encrypted with password if keyring unavailable)
- Recovers configuration if database exists but master key is corrupted
- Prompts for provider and API key

Prompts for:

- Provider selection (number or name)
- API key (hidden input)

If a provider already exists, you'll be asked to confirm override.

### reset

Reset all ply configuration, keys, and encrypted data.

```bash
ply reset
```

This command removes:
- All encrypted API keys (database.json and database.json.bak)
- Master key from OS keyring
- Legacy master.key file (if exists)

**Warning:** This action cannot be undone and requires explicit "yes" confirmation.

After reset, run `ply setup` to re-add your providers.

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

### version

Print version information.

```bash
ply version
```

Shows version, commit, and build date.

### pi

Manage pi installation and status.

```bash
ply pi
```

Shows pi installation commands and status.

### pi install

Install pi coding agent if not already installed.

```bash
ply pi install
```

This command:
- Checks which package manager is available (npm, pnpm, yarn, or bun)
- Installs `@mariozechner/pi-coding-agent` globally
- Shows installation success and version
- Skips if pi is already installed

Note: If pi is already installed, you'll see the location and can reinstall using:
```bash
go uninstall ply && ply pi install
```

### models

List supported AI models for all providers.

```bash
ply models
```

This command fetches the latest model list from the pi-mono repository:
- amazon-bedrock
- anthropic
- azure-openai-responses
- cerebras
- github-copilot
- google
- google-antigravity
- google-gemini-cli
- google-vertex
- groq
- huggingface
- kimi-coding
- minimax
- minimax-cn
- mistral
- openai
- openai-codex
- opencode
- openrouter
- vercel-ai-gateway
- xai
- zai

Models are cached locally for 24 hours.
