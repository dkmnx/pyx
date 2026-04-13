# Storage Module

Data persistence layer for pyx.

## Overview

Handles reading and writing encrypted data files including provider credentials, settings, and caches.

## Files

| File               | Description                       |
| ------------------ | --------------------------------- |
| `mod.rs`           | Module exports                    |
| `database.rs`      | Encrypted provider credentials    |
| `paths.rs`         | Platform-specific path management |
| `settings.rs`      | User preferences                  |
| `providers_env.rs` | Provider environment variables    |
| `models_cache.rs`  | Cached model list                 |
| `atomic_write.rs`  | Safe file writes                  |

## Data Files

All data is stored in `~/.local/share/pyx/`:

| File             | Purpose              | Permissions   |
| ---------------- | -------------------- | ------------- |
| `master.key`     | Encrypted master key | 0600          |
| `database.json`  | Encrypted API keys   | 0600          |
| `models.json`    | Cached model list    | 0644          |
| `providers.json` | Custom mappings      | 0644          |
| `settings.json`  | User settings        | 0644          |

## Security

- Files use restrictive permissions (0600 for secrets)
- All writes are atomic to prevent corruption
- Data is encrypted before storage

## Testing

```bash
just test
```
