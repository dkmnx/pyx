# Storage

Data files, formats, and directory structure for pyx.

## Data Directory

All pyx data is stored in:

```text
~/.local/share/pyx/
```

## File Structure

```mermaid
graph TD
    subgraph DataDir["~/.local/share/pyx/"]
        MK[master.key<br/>0600]
        DB[database.json<br/>0600]
        M[models.json<br/>0644]
        P[providers.json<br/>0644]
        S[settings.json<br/>0644]
        PASS[.passphrase<br/>0600]
    end
```

## File Descriptions

### master.key

Encrypted master key file.

| Property    | Value                |
| ----------- | -------------------- |
| Format      | age-encrypted binary |
| Permissions | 0600                 |
| Created     | During `pyx init`    |

### database.json

Encrypted provider API keys.

| Property    | Value                             |
| ----------- | --------------------------------- |
| Format      | age-encrypted JSON                |
| Permissions | 0600                              |
| Contents    | Provider name → encrypted API key |

**Structure (before encryption):**

```json
[
  {
    "provider": "openai",
    "cipher": "age-encrypted-base64-ciphertext...",
    "created_at": "2026-01-01T00:00:00Z",
    "updated_at": "2026-01-01T00:00:00Z"
  }
]
```

The `cipher` field contains a base64-encoded age ciphertext (ChaCha20-Poly1305 encrypted with the master key).

### models.json

Cached list of supported AI models.

| Property    | Value                 |
| ----------- | --------------------- |
| Format      | JSON                  |
| Permissions | 0644                  |
| Source      | Remote API or bundled |

### providers.json

Custom provider-to-environment-variable mappings.

| Property    | Value                               |
| ----------- | ----------------------------------- |
| Format      | JSON                                |
| Permissions | 0644                                |
| Location    | `~/.local/share/pyx/providers.json` |

**Example:**

```json
{
  "openai": "CUSTOM_OPENAI_KEY"
}
```

### settings.json

User preferences and configuration.

| Property    | Value   |
| ----------- | ------- |
| Format      | JSON    |
| Permissions | 0644    |

### .passphrase

Encrypted passphrase (fallback storage).

| Property    | Value                  |
| ----------- | ---------------------- |
| Format      | Machine-encrypted      |
| Permissions | 0600                   |
| Used when   | OS keyring unavailable |

### Atomic Writes

All sensitive files (`master.key`, `database.json`, `.passphrase`) are written atomically using a temporary file and rename. A `.bak` backup of the existing file is created before writing and **removed after the rename succeeds**. This order ensures the original file always survives a crash, but backups are not kept on disk afterward since they contain sensitive data.

## Directory Permissions

```bash
# Secure directory
chmod 700 ~/.local/share/pyx

# Secure files
chmod 600 ~/.local/share/pyx/master.key
chmod 600 ~/.local/share/pyx/database.json
chmod 600 ~/.local/share/pyx/.passphrase
```

## Backup

To backup your configuration:

```bash
# Backup data directory
tar -czf pyx-backup.tar.gz ~/.local/share/pyx/

# Restore
tar -xzf pyx-backup.tar.gz -C ~/
```

**Note:** Backups contain encrypted data. Without the passphrase, backups cannot be decrypted.
