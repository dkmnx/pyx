# Providers Module

Provider handling and environment variable mapping.

## Overview

Maps provider names to environment variables with support for built-in and custom mappings.

## Files

| File            | Description                  |
| --------------- | ---------------------------- |
| `mod.rs`        | Module exports               |
| `mapping.rs`    | Provider-to-env-var mappings |
| `validation.rs` | API key validation           |

## Built-in Providers

The module includes mappings for 25+ providers including:

- Core: openai, anthropic, google, groq, mistral
- Chinese: deepseek, minimax, minimax-cn, qwen
- Cloud: amazon-bedrock, azure-openai-responses, google-vertex

## Custom Providers

Custom mappings can be added via `providers.json`:

```json
{
  "my-provider": "MY_CUSTOM_API_KEY"
}
```

## Resolution Order

1. `providers.json` - Custom mappings
2. Built-in mappings
3. Name derivation (`provider` → `PROVIDER_API_KEY`)

## Testing

```bash
just test
```

## Adding a Provider

1. Add mapping to `mapping.rs`
2. Update tests in `mapping.rs`
