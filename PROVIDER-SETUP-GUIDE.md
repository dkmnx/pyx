# Provider Setup Guide

## Quick Start

### Step 1: Initialize Pyx

```bash
pyx setup
```

This will:

- Generate a master encryption key
- Store passphrase in your OS keyring
- Create encrypted database

### Step 2: Add Providers

```bash
# Add a provider interactively
pyx add openai
# Enter your API key when prompted

# Add more providers
pyx add anthropic
pyx add google-vertex
```

### Step 3: Verify

```bash
# List all configured providers
pyx list

# Or in JSON format
pyx list --json
```

### Step 4: Use with Pi

```bash
# Run with all providers
pyx

# Run with specific provider
pyx openai

# Run with session
pyx -s <session-uuid>
pyx openai -s <session-uuid>
```

## Available Commands

| Command | Description |
|---------|-------------|
| `pyx setup` | Initialize pyx (run once) |
| `pyx add <name>` | Add a new provider |
| `pyx list` | List all providers |
| `pyx delete <name>` | Remove a provider |
| `pyx reset` | Delete all data |

## Supported Providers

### Core Providers

- `openai` - OpenAI API
- `anthropic` - Anthropic API
- `google` - Google API
- `google-vertex` - Google Vertex AI
- `azure` - Azure OpenAI
- `azure-openai` - Azure OpenAI (alternate)

### Chinese Providers

- `minimax` / `minimax-cn` - MiniMax
- `zhipu` - Zhipu AI
- `baichuan` - Baichuan AI
- `moonshot` - Moonshot AI

### Other Providers

- `groq` - Groq Cloud
- `mistral` - Mistral AI
- `cohere` - Cohere
- `together` - Together AI
- `anyscale` - Anyscale
- `replicate` - Replicate
- `perplexity` - Perplexity
- `friendli` - FriendliAI
- `vercel-ai` - Vercel AI Gateway
- `vercel-openai` - Vercel OpenAI
- `vercel-anthropic` - Vercel Anthropic

## Adding Custom Providers

For extension-backed or custom providers:

1. Add the provider normally:

   ```bash
   pyx add my-custom-provider
   ```

2. The environment variable will be auto-derived:
   - `my-custom-provider` → `MY_CUSTOM_PROVIDER_API_KEY`

3. Or specify custom mapping in `~/.local/share/ply/providers.json`:

   ```json
   {
     "schemaVersion": 1,
     "providers": [
       {
         "name": "my-custom-provider",
         "envVar": "CUSTOM_ENV_VAR_NAME"
       }
     ]
   }
   ```

## Security Notes

- API keys are encrypted with AES-256-GCM
- Passphrase stored in OS keyring (secure enclave/Keychain/SecretService)
- Database file has 0600 permissions (owner read/write only)
- Keys are zeroed from memory after use

## Troubleshooting

### "Pyx not initialized"

Run `pyx setup` first.

### "Provider already exists"

Use `pyx delete <name>` first, then re-add.

### "No passphrase available"

Check your OS keyring or set `PLY_PASSPHRASE` environment variable.

### API Key Not Working

1. Verify the key is correct (no extra spaces)
2. Check provider name matches exactly
3. Try `pyx list` to confirm it's configured
4. Check pi is installed: `which pi`

## Migration from Go Version

If you're switching from the Go implementation:

1. **Current State**: Rust version can read Go database format
2. **Adding New Providers**: Use Go version until migration tool is ready
3. **Future**: Migration tool will convert all data to Rust-native format

See `RUST-IMPLEMENTATION-SUMMARY.md` for migration timeline.

## Example Session

```bash
$ pyx setup
=== Pyx Setup ===

This will initialize pyx with secure encrypted storage.

Enter a passphrase to encrypt your API keys:
(This will be stored in your OS keyring)

Passphrase: ****************
Confirm passphrase: ****************

Generating master key...
Storing passphrase in OS keyring...
Saving encrypted master key...
Initializing provider database...

✓ Setup complete!

$ pyx add openai
Enter API key for provider: openai
(The key will be encrypted and stored securely)

API Key: sk-****************************************

✓ Provider 'openai' configuration prepared!

$ pyx list
Configured providers:
  - openai

Total: 1 provider(s)

$ pyx openai
✓ Loaded provider: openai

Launching pi with 1 provider(s)...
[pi starts with OPENAI_API_KEY set]
```

## Next Steps

After setup:

1. Add your primary AI provider(s)
2. Update models cache: `pyx models update`
3. Install shell completions for better UX
4. Start using pi with your configured providers!
