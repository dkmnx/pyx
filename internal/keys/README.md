# Keys Package

The `keys` package provides secure master key storage for the ply CLI tool.

## Overview

This package addresses a critical security vulnerability where the master encryption key was previously stored in plaintext on disk. The new implementation uses a tiered approach:

1. **Primary**: OS keyring/keychain storage (using `github.com/zalando/go-keyring`)
2. **Fallback**: Password-based encryption using Argon2 KDF

## Design

### Keyring Storage (Primary)

On systems with available keyring/keychain services (macOS Keychain, Windows Credential Manager, Linux Secret Service), the master key is stored securely without requiring user interaction after initial setup.

**Benefits:**
- No password entry required during normal operation
- Uses platform-specific secure storage
- Keys are automatically encrypted by the OS

### Password-Based Storage (Fallback)

When keyring access is unavailable (common in SSH sessions, containers, or headless environments), the master key is encrypted using a user-provided password.

**Security features:**
- Argon2id KDF for key derivation (memory-hard, resistant to GPU/ASIC attacks)
  - Time cost: 3 iterations
  - Memory cost: 64 MB
  - Parallelism: 4 threads
- Random 32-byte salt per installation
- Verification hash to detect incorrect passwords
- Encrypted using XOR with derived key (simple but secure when combined with strong KDF)

## Migration

Existing users will need to re-run `ply setup` after upgrading. The old plaintext master key file (`master.key`) will be ignored, and the system will create a new secure key.

**Migration steps:**
1. Run `ply setup`
2. If keyring is available, it will be used automatically
3. If keyring is unavailable, you'll be prompted to set a password
4. Re-add your provider credentials

**Important**: The old plaintext `master.key` file is not automatically removed. Users should manually delete it after migration:
```bash
rm ~/.local/share/ply/master.key
# or
rm $XDG_DATA_HOME/ply/master.key
```

## API

### Main Types

```go
type Manager struct {
    dataDir string
}
```

### Key Functions

#### `New(dataDir string) *Manager`
Creates a new Manager instance.

#### `Save(key []byte) error`
Saves the master key securely. Attempts keyring storage first, falls back to password-based encryption.

#### `Load(password []byte) ([]byte, error)`
Loads the master key. Password is only required for password-based storage.

#### `Exists() (bool, error)`
Checks if a master key exists in storage.

#### `RequiresPassword() (bool, error)`
Returns true if the stored key requires a password (i.e., file-based storage is in use).

#### `Delete() error`
Removes the master key from all storage locations.

#### `SetPassword(password []byte) error`
Sets or updates the password for key derivation (used when keyring is unavailable).

#### `GenerateKey() ([]byte, error)`
Generates a new random 256-bit master key.

## Usage Example

```go
import (
    "github.com/dkmnx/ply/internal/keys"
    "github.com/dkmnx/ply/internal/fs"
)

func setup() error {
    dataDir, err := fs.EnsureDataDir()
    if err != nil {
        return err
    }

    keyMgr := keys.New(dataDir)

    // Check if key exists
    exists, err := keyMgr.Exists()
    if err != nil {
        return err
    }

    var masterKey []byte
    if exists {
        // Load existing key (password may be required)
        requiresPassword, _ := keyMgr.RequiresPassword()
        var password []byte
        if requiresPassword {
            password = []byte(getUserPassword())
        }
        masterKey, err = keyMgr.Load(password)
        if err != nil {
            return err
        }
    } else {
        // Generate and save new key
        masterKey, err = keys.GenerateKey()
        if err != nil {
            return err
        }
        err = keyMgr.Save(masterKey)
        if err != nil {
            return err
        }
    }

    // Use masterKey for encryption...
    return nil
}
```

## Security Considerations

### Keyring Mode
- Relies on OS-provided security
- Requires proper system security (user account with password/keychain access)
- Keyring credentials may be accessible to other processes running as the same user

### Password Mode
- Password strength is critical - recommend using a strong, unique password
- Argon2 parameters balance security and performance
- 64 MB memory cost provides good security against GPU-based attacks
- Verification hash prevents timing attacks on password validation
- If password is lost, all stored API keys become irrecoverable

## File Locations

### Keyring Mode
No files are created. The key is stored in the OS keychain/keyring.

### Password Mode
- `~/.local/share/ply/master-key.json` - Encrypted master key with metadata
- `~/.local/share/ply/password.bin` - User password (0600 permissions)

The password file contains the raw password in plaintext, protected only by file permissions. This is intentional - the password is used to derive the encryption key each time. On Linux systems, you can use `tmpfs` or encrypted home directories for additional protection.

## Future Improvements

Potential enhancements:
1. Support for hardware security modules (HSMs) and YubiKey
2. Integration with system password managers (1Password, Bitwarden, etc.)
3. Option for password-less keyring with biometric unlock (Touch ID, Windows Hello)
4. Automatic cleanup of old plaintext master key files during migration
