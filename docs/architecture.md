# Architecture

System design and security model.

## Components

```text
src/
├── main.rs          # CLI entry point
├── lib.rs           # Library exports
├── cli.rs           # Command definitions
├── commands/        # Command implementations
├── storage/         # Data persistence
├── keys/            # Key management
├── crypto/          # Encryption
├── providers/       # Provider handling
├── models/          # Model fetching
└── pi/              # Pi integration
```

## Data Flow

```text
User → pyx CLI → Load master key → Decrypt database → Set env vars → Execute pi
```

## Security Model

### Encryption

- **Algorithm**: Age encryption (scrypt + AES-256-GCM)
- **Master Key**: 32 random bytes, encrypted with passphrase
- **Passphrase**: Stored in OS keyring, fallback to env var

### File Permissions

| File            | Permission   |
| --------------- | ------------ |
| `master.key`    | 0600         |
| `database.json` | 0600         |

### Secret Handling

- Keys zeroized after use
- No plaintext in logs
- Memory-secure types where possible

## Provider Resolution

1. `providers.json` - Explicit mappings
2. `settings.json` - Legacy mappings
3. Built-in mappings (50+ providers)
4. Derived: `provider` → `PROVIDER_API_KEY`

## Storage

| File             | Purpose              |
| ---------------- | -------------------- |
| `master.key`     | Encrypted master key |
| `database.json`  | Encrypted API keys   |
| `models.json`    | Cached model list    |
| `settings.json`  | User settings        |
| `providers.json` | Custom mappings      |
