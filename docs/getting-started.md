# Getting Started

## Prerequisites

- Rust 1.75+ and Cargo
- OS keyring support (libsecret on Linux, Keychain on macOS, Credential Manager on Windows)
- API key for your chosen provider

## Installation

```bash
git clone https://github.com/dkmnx/pyx.git
cd pyx
cargo install --path .
```

## Setup

```bash
pyx setup
```

This will:

1. Create the data directory (`~/.local/share/pyx/`)
2. Generate a master encryption key
3. Store passphrase in OS keyring
4. Prompt for provider and API key

Run `pyx setup` again to add or edit providers.

## Supported Providers

**Core:** openai, anthropic, google, google-vertex, azure, azure-openai

**Chinese:** minimax, minimax-cn, zhipu, baichuan, moonshot

**Other:** groq, mistral, cohere, together, anyscale, replicate, perplexity, friendli, vercel-ai

## Custom Providers

Add to `~/.local/share/pyx/providers.json`:

```json
{
  "my-provider": "MY_PROVIDER_API_KEY"
}
```

## Directory Structure

```text
~/.local/share/pyx/
├── master.key      # Encrypted master key (0600)
├── database.json   # Encrypted provider credentials
├── models.json     # Cached model list
└── providers.json  # Custom provider mappings (optional)
```

## Next Steps

- [Usage Guide](usage.md) - Complete command reference
- [Troubleshooting](troubleshooting.md) - Common issues
