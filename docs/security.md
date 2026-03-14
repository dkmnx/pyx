# Security

This document explains the encryption design, cryptographic parameters, and security considerations for pyx.

## Encryption Design

pyx uses **age encryption** with **scrypt** passphrase-based key derivation to protect your API keys and master key.

### Key Hierarchy

1. **Passphrase**: Your master passphrase (stored in OS keyring or environment variable)
2. **Master Key**: 32-byte random key generated during initial setup
3. **Provider Keys**: Individual API keys for each AI provider

### Encryption Flow

Passphrase -> scrypt KDF -> Encrypted Master Key
Master Key -> age encryption -> Encrypted Provider API Keys

## Cryptographic Parameters

### Scrypt Parameters (Passphrase -> Master Key)

| Parameter           | Default        | Configurable             | Range      |
| ------------------- | -------------- | ------------------------ | ---------- |
| Work Factor (N)     | 2^18 (262,144) | `PYX_SCRYPT_WORK_FACTOR` | 14-30      |
| Salt Length         | 16 bytes       | `PYX_SCRYPT_SALT_LEN`    | 8-32 bytes |
| Block Size (r)      | 8              | No                       | Fixed      |
| Parallelization (p) | 1              | No                       | Fixed      |
| Output Key Length   | 32 bytes       | No                       | Fixed      |

### Tuning Parameters

For most users, the default parameters provide strong security with acceptable performance. However, you can adjust them based on your threat model:

**Higher Security (slower):**

```bash
export PYX_SCRYPT_WORK_FACTOR=20  # 2^20 iterations (~4x slower, much stronger)
export PYX_SCRYPT_SALT_LEN=32     # Maximum salt length
```

**Faster Performance (lower security):**

```bash
export PYX_SCRYPT_WORK_FACTOR=16  # 2^16 iterations (~8x faster, weaker)
```

**Warning**: Lower work factors make brute-force attacks more feasible. Only reduce if necessary for performance reasons.

## Security Properties

### What's Protected

**Encrypted at rest:**

- Master key (encrypted with passphrase)
- All provider API keys (encrypted with master key)
- Passphrase file fallback (encrypted with machine-derived key)

**Protected against:**

- Unauthorized file access (0600 permissions)
- Casual inspection (encrypted data)
- Offline brute-force (scrypt work factor)
- Machine theft (machine-bound encryption for passphrase file)

### What's NOT Protected

**Not protected against:**

- Memory scraping while pyx is running
- Keyloggers capturing your passphrase
- Malware with user-level access
- Weak passphrases with low work factor

## Best Practices

### Strong Passphrase

Use a **strong, unique passphrase**:

- Minimum 12 characters (20+ recommended)
- Mix of uppercase, lowercase, numbers, symbols
- Avoid common words or patterns
- Consider using a password manager

Example: `correct-horse-battery-staple-42!`

### OS Keyring

pyx automatically stores your passphrase in the **OS keyring** when available:

- **Linux**: Secret Service API (GNOME Keyring, KWallet)
- **macOS**: Keychain
- **Windows**: Credential Manager

This provides an additional layer of protection beyond file encryption.

### Environment Variable Fallback

For automation or headless environments, you can use:

```bash
export PYX_PASSPHRASE="your-secure-passphrase"
```

**Warning**: Environment variables may be visible in process listings and shell history. Use with caution.

### Regular Updates

Keep pyx updated to receive security patches:

```bash
cd pyx && just install
```

## File Permissions

All pyx data files use restrictive permissions:

- **master.key**: 0600 (owner read/write only)
- **database.json**: 0600 (owner read/write only)
- **.passphrase**: 0600 (owner read/write only)

## Audit Trail

### Logging

pyx does **not** log:

- Passphrases
- API keys
- Decrypted secrets

Debug output may include:

- Provider names
- Operation timestamps
- Error messages (without secrets)

### Data Location

All pyx data is stored in:

```text
~/.local/share/pyx/
├── master.key       # Encrypted master key
├── database.json    # Encrypted provider credentials
└── .passphrase      # Encrypted passphrase (fallback)
```

## Incident Response

### If You Suspect Compromise

1. **Immediately revoke** all API keys in your provider dashboards
2. **Reset pyx**: `pyx reset`
3. **Generate new API keys** and re-run `pyx setup`
4. **Change your passphrase** to a new, unique value

### If You Forget Your Passphrase

Unfortunately, pyx cannot recover your data without the passphrase:

```bash
# Reset everything and start fresh
pyx reset
```

This will delete all encrypted data. You'll need to re-enter your API keys.

## Implementation Details

### Age Encryption

pyx uses the [age](https://age-encryption.org/) encryption tool:

- Modern, secure encryption design
- Scrypt passphrase support
- Armor (base64) encoding for file storage
- Compatible with Go age implementation

### Scrypt Key Derivation

Scrypt is a password-based key derivation function designed to be:

- **Memory-hard**: Resistant to GPU/ASIC attacks
- **Configurable**: Work factor adjusts CPU/memory cost
- **Salted**: Unique salt prevents rainbow table attacks

### Machine-Bound Encryption (Fallback)

The passphrase file fallback (.passphrase) uses a machine-derived key:

- Combines /etc/machine-id (Linux) or OS info
- Includes username for multi-user systems
- Provides some protection against data theft

This is **less secure** than OS keyring but ensures persistence on systems where keyring is unreliable.

## Security Updates

Security vulnerabilities should be reported responsibly. See [CONTRIBUTING.md](contributing.md) for disclosure guidelines.

## References

- [age encryption documentation](https://age-encryption.org/)
- [scrypt paper (Colin Percival, 2009)](https://www.tarsnap.com/scrypt/scrypt.pdf)
- [NIST Password Guidelines (SP 800-63B)](https://pages.nist.gov/800-63-3/sp800-63b.html)
