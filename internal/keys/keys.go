package keys

import (
	"context"
	"crypto/rand"
	"crypto/sha256"
	"crypto/subtle"
	"encoding/base64"
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"time"

	"github.com/dkmnx/ply/internal/crypto"
	"github.com/dkmnx/ply/internal/database"
	"github.com/zalando/go-keyring"
	"golang.org/x/crypto/argon2"
)

const (
	keySize         = 32        // 256 bits for AES-256
	saltSize        = 32        // Salt size for KDF
	argon2Time      = 3         // Number of iterations
	argon2Memory    = 64 * 1024 // 64 MB
	argon2Threads   = 4
	argon2KeyLength = keySize
	keyringService  = "ply"
	keyringUser     = "master-key"
)

var (
	// ErrKeyNotFound is returned when the master key is not found.
	ErrKeyNotFound = errors.New("master key not found")

	// ErrInvalidKeyData is returned when the stored key data is corrupted.
	ErrInvalidKeyData = errors.New("invalid key data")

	// ErrKeyringUnavailable is returned when keyring storage is unavailable.
	ErrKeyringUnavailable = errors.New("keyring unavailable")

	// ErrPasswordRequired is returned when a password is needed for key derivation.
	ErrPasswordRequired = errors.New("password required")

	// ErrInvalidPassword is returned when the provided password is incorrect.
	ErrInvalidPassword = errors.New("invalid password")
)

// KeyData represents stored key data for password-based storage.
type KeyData struct {
	Key           string `json:"key"`
	Salt          string `json:"salt"`
	VerifyHash    string `json:"verify_hash"`
	Argon2Time    uint32 `json:"argon2_time"`
	Argon2Memory  uint32 `json:"argon2_memory"`
	Argon2Threads uint8  `json:"argon2_threads"`
	Argon2KeyLen  uint32 `json:"argon2_key_len"`
}

// Manager handles secure master key storage.
type Manager struct {
	dataDir string
}

// New creates a new Manager instance.
func New(dataDir string) *Manager {
	return &Manager{
		dataDir: dataDir,
	}
}

// MigrateFromLegacy attempts to migrate a legacy plaintext master key to secure storage.
// It checks for the old master.key file and if found:
// 1. Decrypts existing database entries with the legacy key
// 2. Re-encrypts them with the new key (existing or newly generated)
// 3. Saves the updated database
// 4. Removes the legacy key file
// Returns true if migration was attempted (regardless of success), false if no legacy key was found.
func (m *Manager) MigrateFromLegacy(db *database.Database) (bool, error) {
	// Check for legacy master key file
	legacyKeyPath := filepath.Join(m.dataDir, "master.key")
	legacyKey, err := os.ReadFile(legacyKeyPath)
	if err != nil {
		if os.IsNotExist(err) {
			// No legacy key found, nothing to migrate
			return false, nil
		}
		return false, fmt.Errorf("failed to read legacy master key: %w", err)
	}

	// Validate legacy key format (should be 32 bytes for AES-256)
	if len(legacyKey) != keySize {
		return false, fmt.Errorf("invalid legacy master key size: expected %d bytes, got %d", keySize, len(legacyKey))
	}

	// Determine the new key to use
	newKey, useLegacyAsNew, err := m.getMigrationKey(legacyKey)
	if err != nil {
		return false, fmt.Errorf("failed to determine migration key: %w", err)
	}

	// Load database
	if err := db.Load(context.Background()); err != nil {
		return false, fmt.Errorf("failed to load database for migration: %w", err)
	}

	// Check if there are any entries to migrate
	entries := db.ListEntries()
	if len(entries) == 0 {
		return m.migrateKeyOnly(legacyKey, newKey, useLegacyAsNew, legacyKeyPath)
	}

	return m.migrateDatabaseEntries(legacyKey, newKey, useLegacyAsNew, legacyKeyPath, db, entries)
}

// migrateKeyOnly migrates the legacy key without any database entries.
func (m *Manager) migrateKeyOnly(legacyKey, newKey []byte, useLegacyAsNew bool, legacyKeyPath string) (bool, error) {
	fmt.Fprintln(os.Stderr, "Migrating master key to secure storage...")
	if useLegacyAsNew {
		if err := m.Save(newKey); err != nil {
			return false, fmt.Errorf("failed to save master key to secure storage: %w", err)
		}
	}
	if err := os.Remove(legacyKeyPath); err != nil {
		fmt.Fprintf(os.Stderr, "Warning: could not remove legacy master key file: %v\n", err)
	}
	return true, nil
}

// migrateDatabaseEntries migrates database entries from legacy key to new key.
func (m *Manager) migrateDatabaseEntries(legacyKey, newKey []byte, useLegacyAsNew bool, legacyKeyPath string, db *database.Database, entries []database.Entry) (bool, error) {
	fmt.Fprintf(os.Stderr, "Migrating %d provider(s) to new key encryption...\n", len(entries))
	successCount, failedCount := m.migrateEntries(legacyKey, newKey, db, entries)

	// Save the updated database
	if successCount > 0 || failedCount > 0 {
		if err := db.Save(context.Background()); err != nil {
			return false, fmt.Errorf("failed to save database after migration: %w", err)
		}
	}

	// Save the new key if we're using the legacy key
	if useLegacyAsNew {
		if err := m.Save(newKey); err != nil {
			return false, fmt.Errorf("failed to save master key to secure storage: %w", err)
		}
	}

	// Remove legacy key file
	if err := os.Remove(legacyKeyPath); err != nil {
		fmt.Fprintf(os.Stderr, "Warning: could not remove legacy master key file: %v\n", err)
	}

	// Report results
	m.reportMigrationResults(successCount, failedCount)
	return true, nil
}

// migrateEntries decrypts and re-encrypts each database entry.
func (m *Manager) migrateEntries(legacyKey, newKey []byte, db *database.Database, entries []database.Entry) (int, int) {
	successCount := 0
	failedCount := 0

	for i := range entries {
		entry := &entries[i]
		if m.migrateSingleEntry(legacyKey, newKey, db, entry) {
			successCount++
		} else {
			failedCount++
		}
	}

	return successCount, failedCount
}

// migrateSingleEntry attempts to decrypt and re-encrypt a single entry.
// Returns true on success, false on failure.
func (m *Manager) migrateSingleEntry(legacyKey, newKey []byte, db *database.Database, entry *database.Entry) bool {
	// Try to decrypt with legacy key
	apiKey, err := crypto.Decrypt(legacyKey, entry.Cipher, entry.Nonce)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Warning: failed to decrypt %s with legacy key: %v\n", entry.Provider, err)
		// Zero any decrypted data that might have leaked
		if apiKey != nil {
			apiKey.Zero()
		}
		return false
	}

	// Re-encrypt with new key
	newCipher, newNonce, err := crypto.Encrypt(newKey, apiKey.String())
	// Zero the decrypted API key immediately
	apiKey.Zero()
	if err != nil {
		fmt.Fprintf(os.Stderr, "Warning: failed to re-encrypt %s with new key: %v\n", entry.Provider, err)
		return false
	}

	// Update entry
	entry.Cipher = newCipher
	entry.Nonce = newNonce
	entry.UpdatedAt = time.Now().UTC()
	if err := db.UpdateEntry(*entry); err != nil {
		fmt.Fprintf(os.Stderr, "Warning: failed to update %s in database: %v\n", entry.Provider, err)
		return false
	}

	return true
}

// reportMigrationResults prints the migration summary.
func (m *Manager) reportMigrationResults(successCount, failedCount int) {
	if failedCount > 0 {
		fmt.Fprintf(os.Stderr, "Migration completed with warnings: %d succeeded, %d failed\n", successCount, failedCount)
		fmt.Fprintln(os.Stderr, "Failed providers will need to be re-added using 'ply setup'")
	} else {
		fmt.Fprintf(os.Stderr, "Migration completed successfully: %d provider(s) migrated\n", successCount)
	}
}

// getMigrationKey determines which key to use for migration.
// Returns (key, useLegacyAsNew, error).
func (m *Manager) getMigrationKey(legacyKey []byte) ([]byte, bool, error) {
	// Check if keyring key exists
	keyringExists, err := m.keyringKeyExists()
	if err != nil {
		return nil, false, fmt.Errorf("failed to check keyring: %w", err)
	}

	if !keyringExists {
		// No keyring key, use legacy as the new key
		return legacyKey, true, nil
	}

	// Keyring key exists, load it
	keyringKey, err := m.loadFromKeyring()
	if err != nil {
		return nil, false, fmt.Errorf("failed to load keyring key: %w", err)
	}

	// Check if legacy and keyring keys are the same
	if Equal(legacyKey, keyringKey) {
		// Same key, no migration needed, use keyring key
		return keyringKey, false, nil
	}

	// Different keys - check which one can decrypt the database
	db := database.New(m.dataDir)
	if err := db.Load(context.Background()); err != nil {
		// Can't load database, prefer legacy (it's the source of truth)
		fmt.Fprintf(os.Stderr, "Warning: cannot load database, using legacy key for migration\n")
		return legacyKey, true, nil
	}

	entries := db.ListEntries()
	if len(entries) == 0 {
		// No entries, use keyring key (newer)
		return keyringKey, false, nil
	}

	// Try to decrypt first entry with keyring key
	firstEntry := entries[0]
	_, err = crypto.Decrypt(keyringKey, firstEntry.Cipher, firstEntry.Nonce)
	if err == nil {
		// Keyring key works, use it
		return keyringKey, false, nil
	}

	// Keyring key doesn't work, legacy key is the correct one
	fmt.Fprintf(os.Stderr, "Warning: existing keyring key cannot decrypt database, using legacy key\n")
	return legacyKey, true, nil
}

// keyringKeyExists checks if a key exists in the OS keyring.
func (m *Manager) keyringKeyExists() (bool, error) {
	_, err := keyring.Get(keyringService, keyringUser)
	if err == nil {
		return true, nil
	}
	if errors.Is(err, keyring.ErrNotFound) {
		return false, nil
	}
	return false, err
}

// Save saves the master key securely.
// It attempts to use keyring storage first. If that fails, it uses password-based storage.
func (m *Manager) Save(key []byte) error {
	// Try keyring storage first
	if err := m.saveToKeyring(key); err == nil {
		return nil
	}

	// Keyring failed, fall back to password-based storage
	return m.saveToFile(key)
}

// Load loads the master key securely.
// It attempts to use keyring storage first. If that fails, it falls back to password-based storage.
// For password-based storage, the password must be provided.
func (m *Manager) Load(password []byte) ([]byte, error) {
	// Try keyring storage first
	key, err := m.loadFromKeyring()
	if err == nil {
		return key, nil
	}

	// Keyring failed, try password-based storage
	return m.loadFromFile(password)
}

// Delete removes the master key from storage.
func (m *Manager) Delete() error {
	// Try to delete from keyring
	if err := keyring.Delete(keyringService, keyringUser); err != nil {
		// Keyring deletion failed - this is not critical, continue with file deletion
		fmt.Fprintf(os.Stderr, "Warning: could not delete key from OS keyring: %v\n", err)
	}

	// Try to delete from file
	filePath := m.keyFilePath()
	if _, err := os.Stat(filePath); err == nil {
		if err := os.Remove(filePath); err != nil {
			return fmt.Errorf("failed to delete key file: %w", err)
		}
	}

	return nil
}

// Exists checks if a master key exists in storage.
func (m *Manager) Exists() (bool, error) {
	// Check keyring first
	_, err := keyring.Get(keyringService, keyringUser)
	if err == nil {
		return true, nil
	}

	// Keyring not available or key not found, check file
	filePath := m.keyFilePath()
	_, err = os.Stat(filePath)
	if err == nil {
		return true, nil
	}
	if os.IsNotExist(err) {
		return false, nil
	}

	return false, fmt.Errorf("failed to check key existence: %w", err)
}

// RequiresPassword returns true if the stored key requires a password.
func (m *Manager) RequiresPassword() (bool, error) {
	// Check keyring first
	_, err := keyring.Get(keyringService, keyringUser)
	if err == nil {
		return false, nil
	}

	// Check file
	filePath := m.keyFilePath()
	_, err = os.Stat(filePath)
	if err == nil {
		return true, nil
	}
	if os.IsNotExist(err) {
		return false, nil
	}

	return false, fmt.Errorf("failed to check password requirement: %w", err)
}

// GenerateKey generates a new random master key.
func GenerateKey() ([]byte, error) {
	key := make([]byte, keySize)
	if _, err := rand.Read(key); err != nil {
		return nil, fmt.Errorf("failed to generate key: %w", err)
	}
	return key, nil
}

// saveToKeyring saves the key using OS keyring.
func (m *Manager) saveToKeyring(key []byte) error {
	keyB64 := base64.StdEncoding.EncodeToString(key)
	if err := keyring.Set(keyringService, keyringUser, keyB64); err != nil {
		return fmt.Errorf("%w: %v", ErrKeyringUnavailable, err)
	}
	return nil
}

// loadFromKeyring loads the key from OS keyring.
func (m *Manager) loadFromKeyring() ([]byte, error) {
	keyB64, err := keyring.Get(keyringService, keyringUser)
	if err != nil {
		if errors.Is(err, keyring.ErrNotFound) {
			return nil, ErrKeyNotFound
		}
		return nil, fmt.Errorf("%w: %v", ErrKeyringUnavailable, err)
	}

	key, err := base64.StdEncoding.DecodeString(keyB64)
	if err != nil {
		return nil, fmt.Errorf("%w: %v", ErrInvalidKeyData, err)
	}

	if len(key) != keySize {
		return nil, fmt.Errorf("%w: invalid key size", ErrInvalidKeyData)
	}

	return key, nil
}

// saveToFile saves the key encrypted with a password.
// This is a fallback when keyring is unavailable.
// It prompts for a password or uses an existing password if stored.
func (m *Manager) saveToFile(key []byte) error {
	// Check if password file exists
	passwordFile := m.passwordFilePath()
	var password []byte

	if _, err := os.Stat(passwordFile); err == nil {
		// Password file exists, use existing password
		password, err = os.ReadFile(passwordFile)
		if err != nil {
			return fmt.Errorf("failed to read password file: %w", err)
		}
		// Zero password after use
		defer zeroBytes(password)
	} else {
		// No password file, cannot proceed
		return fmt.Errorf("%w: no password file found, run 'ply setup' to initialize", ErrPasswordRequired)
	}

	// Generate salt
	salt := make([]byte, saltSize)
	if _, err := rand.Read(salt); err != nil {
		return fmt.Errorf("failed to generate salt: %w", err)
	}

	// Derive encryption key from password
	derivedKey := argon2.IDKey(password, salt, argon2Time, argon2Memory, argon2Threads, argon2KeyLength)

	// Create a verification hash of the derived key
	hasher := sha256.New()
	hasher.Write(derivedKey)
	verifyHash := hasher.Sum(nil)

	// Encrypt the master key using XOR (simple, but combined with Argon2 provides security)
	encryptedKey := make([]byte, keySize)
	for i := range key {
		encryptedKey[i] = key[i] ^ derivedKey[i]
	}

	// Zero derived key
	zeroBytes(derivedKey)

	// Create key data
	keyData := KeyData{
		Key:           base64.StdEncoding.EncodeToString(encryptedKey),
		Salt:          base64.StdEncoding.EncodeToString(salt),
		VerifyHash:    base64.StdEncoding.EncodeToString(verifyHash),
		Argon2Time:    argon2Time,
		Argon2Memory:  argon2Memory,
		Argon2Threads: argon2Threads,
		Argon2KeyLen:  argon2KeyLength,
	}

	// Save to file
	filePath := m.keyFilePath()
	data, err := json.MarshalIndent(keyData, "", "  ")
	if err != nil {
		return fmt.Errorf("failed to marshal key data: %w", err)
	}

	if err := os.MkdirAll(m.dataDir, 0700); err != nil {
		return fmt.Errorf("failed to create data directory: %w", err)
	}

	if err := os.WriteFile(filePath, data, 0600); err != nil {
		return fmt.Errorf("failed to write key file: %w", err)
	}

	return nil
}

// loadFromFile loads the key encrypted with a password.
// This is a fallback when keyring is unavailable.
func (m *Manager) loadFromFile(password []byte) ([]byte, error) {
	if len(password) == 0 {
		return nil, ErrPasswordRequired
	}

	// Zero password after use
	defer zeroBytes(password)

	// Read key data
	filePath := m.keyFilePath()
	data, err := os.ReadFile(filePath)
	if err != nil {
		if os.IsNotExist(err) {
			return nil, ErrKeyNotFound
		}
		return nil, fmt.Errorf("failed to read key file: %w", err)
	}

	// Unmarshal
	var keyData KeyData
	if err := json.Unmarshal(data, &keyData); err != nil {
		return nil, fmt.Errorf("%w: %v", ErrInvalidKeyData, err)
	}

	// Decode encrypted key
	encryptedKey, err := base64.StdEncoding.DecodeString(keyData.Key)
	if err != nil {
		return nil, fmt.Errorf("%w: %v", ErrInvalidKeyData, err)
	}

	// Decode salt
	salt, err := base64.StdEncoding.DecodeString(keyData.Salt)
	if err != nil {
		return nil, fmt.Errorf("%w: %v", ErrInvalidKeyData, err)
	}

	// Derive encryption key from password
	derivedKey := argon2.IDKey(password, salt, keyData.Argon2Time, keyData.Argon2Memory, keyData.Argon2Threads, keyData.Argon2KeyLen)

	// Verify the derived key against the stored hash
	hasher := sha256.New()
	hasher.Write(derivedKey)
	computedHash := hasher.Sum(nil)

	storedHash, err := base64.StdEncoding.DecodeString(keyData.VerifyHash)
	if err != nil {
		return nil, fmt.Errorf("%w: %v", ErrInvalidKeyData, err)
	}

	if !Equal(computedHash, storedHash) {
		zeroBytes(derivedKey)
		return nil, ErrInvalidPassword
	}

	// Decrypt the master key
	masterKey := make([]byte, keySize)
	for i := range encryptedKey {
		masterKey[i] = encryptedKey[i] ^ derivedKey[i]
	}

	// Zero derived key
	zeroBytes(derivedKey)

	return masterKey, nil
}

// SetPassword sets or updates the password for key derivation.
// This is used when keyring is unavailable.
func (m *Manager) SetPassword(password []byte) error {
	if err := os.MkdirAll(m.dataDir, 0700); err != nil {
		return fmt.Errorf("failed to create data directory: %w", err)
	}

	filePath := m.passwordFilePath()
	if err := os.WriteFile(filePath, password, 0600); err != nil {
		return fmt.Errorf("failed to write password file: %w", err)
	}
	return nil
}

// PasswordExists checks if a password has been set.
func (m *Manager) PasswordExists() (bool, error) {
	filePath := m.passwordFilePath()
	_, err := os.Stat(filePath)
	if err == nil {
		return true, nil
	}
	if os.IsNotExist(err) {
		return false, nil
	}
	return false, fmt.Errorf("failed to check password file: %w", err)
}

// keyFilePath returns the path to the key data file.
func (m *Manager) keyFilePath() string {
	return filepath.Join(m.dataDir, "master-key.json")
}

// passwordFilePath returns the path to the password file.
func (m *Manager) passwordFilePath() string {
	return filepath.Join(m.dataDir, "password.bin")
}

// zeroBytes securely zeros a byte slice.
func zeroBytes(b []byte) {
	for i := range b {
		b[i] = 0
	}
}

// Equal securely compares two byte slices in constant time.
func Equal(a, b []byte) bool {
	if len(a) != len(b) {
		return false
	}
	return subtle.ConstantTimeCompare(a, b) == 1
}
