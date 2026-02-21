# Getting Started

This guide covers initial setup and configuration of ply.

## Prerequisites

- Go 1.21 or later
- Access to the pi coding agent
- API key for your chosen provider

## Installation

### From Source

```bash
git clone https://github.com/dkmnx/ply.git
cd ply
make build-prod
sudo mv bin/ply /usr/local/bin/
```

### Via go install

```bash
go install github.com/dkmnx/ply/cmd/ply@latest
```

## Initial Configuration

Run the setup command to initialize ply and add your first provider:

```bash
ply setup
```

The setup process will:

1. Create the data directory (`~/.local/share/ply/`)
2. Generate or load the master encryption key
3. Prompt for provider selection
4. Securely prompt for your API key

If the provider already exists, you'll be asked to confirm override.

### Setup Example

```bash
$ ply setup
✓ Master key initialized
Select a provider:
  1. anthropic
  2. openai
  3. google
  ...
Enter provider number or name: 2
Enter API key: sk-...
✓ API key stored securely
```

## Environment Variables

Ply sets the following environment variables when running pi:

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
| Cerebras | `CEREBRAS_API_KEY` |
| Amazon Bedrock | `AWS_BEARER_TOKEN_BEDROCK` |
| GitHub Copilot | `GITHUB_TOKEN` |
| Google Vertex | `GOOGLE_APPLICATION_CREDENTIALS` |
| OpenAI Codex | `OPENAI_API_KEY` |
| MiniMax CN | `MINIMAX_CN_API_KEY` |

## Directory Structure

After initialization, ply creates the following structure:

```text
~/.local/share/ply/
├── database.json      # Encrypted provider credentials
└── master.key        # AES-256-GCM encryption key (0600)
```

## Next Steps

- [Configure multiple providers](usage.md)
- [Run pi with all providers](usage.md#all-configured-providers)
- [Generate shell completion](usage.md#shell-completion)
