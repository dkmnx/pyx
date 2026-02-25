package keys

import (
	"context"
	"crypto/aes"
	"crypto/cipher"
	"crypto/rand"
	"crypto/subtle"
	"encoding/base64"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"time"

	"github.com/dkmnx/ply/internal/crypto"
	"github.com/dkmnx/ply/internal/database"
	"github.com/zalando/go-keyring"
	"golang.org/x/crypto/argon2"
)

// Cryptographic parameters for key derivation and encryption.
//
// Argon2id parameters chosen based on OWASP recommendations for password hashing.
// These values provide a good balance between security and usability for a CLI tool.
// They are not configurable to ensure consistent security across installations.
//
// Reference: OWASP Password Storage Cheat Sheet (2023)
// - argon2Time: Number of iterations (t)
// - argon2Memory: Memory cost in KiB (m)
// - argon2Threads: Parallelism factor (p)
// - argon2KeyLength: Derived key length in bytes
//
// AES-256-GCM is used for master key encryption:
// - keySize: 32 bytes (256 bits)
// - nonceSize: 12 bytes (96 bits) as recommended for GCM
const (
	keySize         = 32        // 256 bits for AES-256
	saltSize        = 32        // Salt size for KDF
	argon2Time      = 3         // OWASP minimum: 1 iteration (t)
	argon2Memory    = 64 * 1024 // OWASP minimum: 64 MiB (m)
	argon2Threads   = 4         // OWASP minimum: 1 thread (p), 4 for modern CPUs
	argon2KeyLength = keySize   // Match AES-256 key size
	keyringService  = "ply"
	keyringUser     = "master-key"

	nonceSize = 12 // 96 bits for GCM

	// keyringPasswordUser is the keyring item key for password storage
	keyringPasswordUser = "password"

	// Legacy key file name for migration
	LegacyKeyFileName = "master.key"
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
	Nonce         string `json:"nonce"`
	Salt          string `json:"salt"`
	VerifyHash    string `json:"verify_hash"`
	Argon2Time    uint32 `json:"argon2_time"`
	Argon2Memory  uint32 `json:"argon2_memory"`
	Argon2Threads uint8  `json:"argon2_threads"`
	Argon2KeyLen  uint32 `json:"argon2_key_len"`
}

// encrypt encrypts plaintext using AES-GCM with the given key.
// Returns base64-encoded ciphertext and nonce.
func encrypt(key []byte, plaintext []byte) (string, string, error) {
	block, err := aes.NewCipher(key)
	if err != nil {
		return "", "", fmt.Errorf("failed to create cipher: %w", err)
	}

	gcm, err := cipher.NewGCM(block)
	if err != nil {
		return "", "", fmt.Errorf("failed to create GCM: %w", err)
	}

	nonce := make([]byte, nonceSize)
	if _, err := io.ReadFull(rand.Reader, nonce); err != nil {
		return "", "", fmt.Errorf("failed to generate nonce: %w", err)
	}

	ciphertext := gcm.Seal(nil, nonce, plaintext, nil)

	return base64.StdEncoding.EncodeToString(ciphertext), base64.StdEncoding.EncodeToString(nonce), nil
}

// decrypt decrypts ciphertext using AES-GCM with the given key.
// Expects base64-encoded ciphertext and nonce.
func decrypt(key []byte, cipherB64, nonceB64 string) ([]byte, error) {
	ciphertext, err := base64.StdEncoding.DecodeString(cipherB64)
	if err != nil {
		return nil, fmt.Errorf("failed to decode ciphertext: %w", err)
	}

	nonce, err := base64.StdEncoding.DecodeString(nonceB64)
	if err != nil {
		return nil, fmt.Errorf("failed to decode nonce: %w", err)
	}

	if len(nonce) != nonceSize {
		return nil, fmt.Errorf("invalid nonce size: got %d bytes, want %d", len(nonce), nonceSize)
	}

	block, err := aes.NewCipher(key)
	if err != nil {
		return nil, fmt.Errorf("failed to create cipher: %w", err)
	}

	gcm, err := cipher.NewGCM(block)
	if err != nil {
		return nil, fmt.Errorf("failed to create GCM: %w", err)
	}

	plaintext, err := gcm.Open(nil, nonce, ciphertext, nil)
	if err != nil {
		return nil, fmt.Errorf("failed to decrypt: %w", err)
	}

	return plaintext, nil
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
// 1. Verifies the legacy key was actually used to encrypt database data
// 2. Decrypts existing database entries with the legacy key
// 3. Re-encrypts them with the new key (existing or newly generated)
// 4. Saves the updated database
// 5. Removes the legacy key file
// Returns true if migration was attempted (regardless of success), false if no legacy key was found.
func (m *Manager) MigrateFromLegacy(db *database.Database) (bool, error) {
	// Check for legacy master key file
	legacyKeyPath := filepath.Join(m.dataDir, LegacyKeyFileName)
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

	// Load database first to verify the legacy key is actually used to encrypt data
	if err := db.Load(context.Background()); err != nil {
		return false, fmt.Errorf("failed to load database for migration: %w", err)
	}

	// Check if there are any entries to migrate
	entries := db.ListEntries()

	// Only migrate if the legacy key was actually used to encrypt data
	if !m.shouldMigrate(legacyKey, entries) {
		// Legacy key exists but wasn't used to encrypt database data
		// This could be from a build artifact or accidental creation
		// Remove it to prevent future confusion
		if err := os.Remove(legacyKeyPath); err != nil {
			fmt.Fprintf(os.Stderr, "Warning: could not remove unused legacy master key file: %v\n", err)
		} else {
			fmt.Fprintln(os.Stderr, "Warning: found unused legacy master.key file (not used to encrypt database), removed")
		}
		return false, nil
	}

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

// shouldMigrate determines if the legacy key should be migrated.
// It verifies the key was actually used to encrypt database data.
func (m *Manager) shouldMigrate(legacyKey []byte, entries []database.Entry) bool {
	// If there are no entries, no way to verify - assume should migrate
	if len(entries) == 0 {
		return true
	}

	// Try to decrypt first entry with legacy key
	firstEntry := entries[0]
	_, err := crypto.Decrypt(legacyKey, firstEntry.Cipher, firstEntry.Nonce)

	// If decryption succeeds, the legacy key was used to encrypt data
	return err == nil
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
// It uses keyring storage when no password is set.
// If a password has been set, it uses password-based file storage.
// When keyring is unavailable and no password is set, it returns an error.
func (m *Manager) Save(key []byte) error {
	// Check if password exists - if so, use file-based storage
	passwordExists, err := m.PasswordExists()
	if err != nil {
		return fmt.Errorf("failed to check password existence: %w", err)
	}

	if passwordExists {
		// Password set, use file-based storage
		return m.saveToFile(key)
	}

	// No password set - use keyring storage only
	// Don't fall back to file storage as that requires a password
	if err := m.saveToKeyring(key); err != nil {
		return fmt.Errorf("failed to save to keyring: %w", err)
	}

	return nil
}

// Load loads the master key securely.
// It checks if a password exists to determine which storage to use.
// If password exists, uses file-based storage; otherwise tries keyring first.
func (m *Manager) Load(password []byte) ([]byte, error) {
	// Check if password exists to determine storage method
	passwordExists, err := m.PasswordExists()
	if err != nil {
		return nil, fmt.Errorf("failed to check password existence: %w", err)
	}

	if passwordExists {
		// Password exists, use file-based storage
		return m.loadFromFile(password)
	}

	// No password, try keyring first
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
// It retrieves the password from keyring.
func (m *Manager) saveToFile(key []byte) error {
	// Get password from keyring
	passwordStr, err := keyring.Get(keyringService, keyringPasswordUser)
	if err != nil {
		return fmt.Errorf("%w: no password found in keyring, run 'ply setup' to initialize", ErrPasswordRequired)
	}
	password := []byte(passwordStr)
	defer zeroBytes(password)

	// Generate salt
	salt := make([]byte, saltSize)
	if _, err := rand.Read(salt); err != nil {
		return fmt.Errorf("failed to generate salt: %w", err)
	}

	// Derive encryption key from password
	derivedKey := argon2.IDKey(password, salt, argon2Time, argon2Memory, argon2Threads, argon2KeyLength)

	// Encrypt the master key using AES-GCM
	cipher, nonce, err := encrypt(derivedKey, key)
	if err != nil {
		zeroBytes(derivedKey)
		return fmt.Errorf("failed to encrypt master key: %w", err)
	}
	zeroBytes(derivedKey)

	// Create key data
	keyData := KeyData{
		Key:           cipher,
		Nonce:         nonce,
		Salt:          base64.StdEncoding.EncodeToString(salt),
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
	// Get or retrieve password
	loadPassword, err := m.getLoadPassword(password)
	if err != nil {
		return nil, err
	}
	defer zeroBytes(loadPassword)

	// Load and unmarshal key data
	keyData, err := m.loadKeyData()
	if err != nil {
		return nil, err
	}

	// Decrypt the master key
	masterKey, err := decryptMasterKey(loadPassword, keyData)
	if err != nil {
		return nil, err
	}

	return masterKey, nil
}

// loadKeyData reads and unmarshals the encrypted key data file.
func (m *Manager) loadKeyData() (KeyData, error) {
	filePath := m.keyFilePath()
	data, err := os.ReadFile(filePath)
	if err != nil {
		if os.IsNotExist(err) {
			return KeyData{}, ErrKeyNotFound
		}
		return KeyData{}, fmt.Errorf("failed to read key file: %w", err)
	}

	var keyData KeyData
	if err := json.Unmarshal(data, &keyData); err != nil {
		return KeyData{}, fmt.Errorf("%w: %v", ErrInvalidKeyData, err)
	}

	// Check for nonce - if missing, it's using legacy XOR encryption
	if keyData.Nonce == "" {
		return KeyData{}, fmt.Errorf("legacy key format detected, please re-initialize with 'ply init'")
	}

	return keyData, nil
}

// getLoadPassword retrieves password for key loading.
// Returns password slice that caller should zero after use.
func (m *Manager) getLoadPassword(providedPassword []byte) ([]byte, error) {
	if len(providedPassword) > 0 {
		return providedPassword, nil
	}

	// Get password from keyring
	passwordStr, err := keyring.Get(keyringService, keyringPasswordUser)
	if err != nil {
		if errors.Is(err, keyring.ErrNotFound) {
			return nil, ErrPasswordRequired
		}
		return nil, fmt.Errorf("failed to get password from keyring: %w", err)
	}

	return []byte(passwordStr), nil
}

// decryptMasterKey decrypts the master key using password-derived key.
func decryptMasterKey(password []byte, keyData KeyData) ([]byte, error) {
	// Decode salt
	salt, err := base64.StdEncoding.DecodeString(keyData.Salt)
	if err != nil {
		return nil, fmt.Errorf("%w: %v", ErrInvalidKeyData, err)
	}

	// Derive decryption key from password
	derivedKey := argon2.IDKey(password, salt,
		keyData.Argon2Time, keyData.Argon2Memory,
		keyData.Argon2Threads, keyData.Argon2KeyLen)

	// Decrypt master key using AES-GCM
	masterKey, err := decrypt(derivedKey, keyData.Key, keyData.Nonce)
	if err != nil {
		return nil, ErrInvalidPassword
	}

	return masterKey, nil
}

// SetPassword sets or updates the password for key derivation.
// Stores password securely in OS keyring instead of plaintext file.
//
// Security Note: Due to Go's immutable strings, the password string passed
// from the caller (e.g., from prompt.PromptNewPassword) will remain in memory
// after this function returns. This is a known limitation of Go's memory
// model. The OS keyring securely stores the password, but the caller's
// string copy cannot be zeroed. For high-security environments, consider
// using keyring-only storage when available.
//
// The caller should zero the password slice after calling this function
// to ensure it is removed from memory where possible.
func (m *Manager) SetPassword(password []byte) error {
	// Store password in keyring - keyring requires string, so convert at the last moment
	if err := keyring.Set(keyringService, keyringPasswordUser, string(password)); err != nil {
		return fmt.Errorf("failed to store password in keyring: %w", err)
	}
	return nil
}

// PasswordExists checks if a password has been set.
func (m *Manager) PasswordExists() (bool, error) {
	// Check keyring for password
	_, err := keyring.Get(keyringService, keyringPasswordUser)
	if err == nil {
		return true, nil
	}
	if errors.Is(err, keyring.ErrNotFound) {
		return false, nil
	}
	return false, fmt.Errorf("failed to check password: %w", err)
}

// keyFilePath returns the path to the key data file.
func (m *Manager) keyFilePath() string {
	return filepath.Join(m.dataDir, "master-key.json")
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
