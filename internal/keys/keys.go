package keys

import (
	"crypto/rand"
	"crypto/subtle"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"sync"
	"time"

	"github.com/dkmnx/ply/internal/crypto"
	"github.com/zalando/go-keyring"
)

const (
	keySize          = 32 // age scrypt identity size (256 bits)
	keyFileName      = "master.key"
	defaultService   = "ply"
	defaultUser      = "master-key"
	legacyPassphrase = "default" // Used in old versions before keyring
)

var (
	ErrKeyNotFound     = errors.New("master key not found")
	ErrInvalidKeyData  = errors.New("invalid key data")
	ErrInvalidPassword = errors.New("invalid password")
	ErrNoPassword      = errors.New("no password set")
	ErrMigrationNeeded = errors.New("migration from legacy format needed")
	ErrTooManyAttempts = errors.New("too many failed attempts, please wait before retrying")
)

// ErrKeyringUnavailable is returned when the OS keyring is not available.
var ErrKeyringUnavailable = errors.New("keyring unavailable")

// Manager handles secure master key storage.
type Manager struct {
	dataDir           string
	keyringService    string
	keyringUser       string
	failedAttempts    int
	lastFailedAttempt time.Time
	mu                sync.Mutex
}

const (
	maxFailedAttempts = 5
	lockoutDuration   = 30 * time.Second
)

// New creates a new Manager instance with default keyring identifiers.
func New(dataDir string) *Manager {
	return &Manager{
		dataDir:        dataDir,
		keyringService: defaultService,
		keyringUser:    defaultUser,
	}
}

// NewWithKeyring creates a new Manager instance with custom keyring identifiers.
// This is primarily intended for testing to isolate test keyring namespaces.
func NewWithKeyring(dataDir, service, user string) *Manager {
	return &Manager{
		dataDir:        dataDir,
		keyringService: service,
		keyringUser:    user,
	}
}

// Save saves the master key to a file, encrypted with the password stored in keyring.
// If no password is set, it returns ErrNoPassword.
func (m *Manager) Save(key []byte) error {
	passphrase, err := m.getStoredPassword()
	if err != nil {
		return fmt.Errorf("failed to get password: %w", err)
	}

	encrypted, err := crypto.Encrypt(passphrase, string(key))
	if err != nil {
		return fmt.Errorf("failed to encrypt master key: %w", err)
	}

	filePath := m.keyFilePath()
	if err := os.MkdirAll(m.dataDir, 0700); err != nil {
		return fmt.Errorf("failed to create data directory: %w", err)
	}

	if err := os.WriteFile(filePath, []byte(encrypted), 0600); err != nil {
		return fmt.Errorf("failed to write key file: %w", err)
	}

	return nil
}

// Load loads the master key from the file, using the password from keyring or env var.
// If no password is set, it tries the legacy "default" passphrase for migration.
func (m *Manager) Load(password []byte) ([]byte, error) {
	m.mu.Lock()
	defer m.mu.Unlock()

	// Check for rate limiting
	if m.failedAttempts >= maxFailedAttempts {
		timeSinceLastAttempt := time.Since(m.lastFailedAttempt)
		if timeSinceLastAttempt < lockoutDuration {
			return nil, ErrTooManyAttempts
		}
		// Reset failed attempts after lockout period
		m.failedAttempts = 0
	}

	// First try with provided password (or from keyring/env var)
	passphrase, err := m.getPassphrase(password)
	if err != nil {
		// If no password available, try legacy passphrase for migration
		if err == ErrNoPassword {
			return m.loadWithLegacyPassphrase()
		}
		return nil, err
	}

	filePath := m.keyFilePath()
	data, err := os.ReadFile(filePath)
	if err != nil {
		if os.IsNotExist(err) {
			return nil, ErrKeyNotFound
		}
		return nil, fmt.Errorf("failed to read key file: %w", err)
	}

	key, err := crypto.Decrypt(passphrase, string(data))
	if err != nil {
		// Record failed attempt for rate limiting
		m.failedAttempts++
		m.lastFailedAttempt = time.Now()

		// If decryption failed with provided password/env var, try legacy passphrase for migration
		// This handles the case where user has old master.key but set PLY_PASSPHRASE
		if err == crypto.ErrInvalidPassphrase {
			return m.loadWithLegacyPassphrase()
		}
		return nil, fmt.Errorf("failed to decrypt master key: %w", err)
	}

	// Reset failed attempts on success
	m.failedAttempts = 0
	m.lastFailedAttempt = time.Time{}

	return []byte(key), nil
}

// loadWithLegacyPassphrase attempts to decrypt the master key using the legacy passphrase.
// This is needed for migration from the old system that used a hardcoded "default" passphrase.
func (m *Manager) loadWithLegacyPassphrase() ([]byte, error) {
	filePath := m.keyFilePath()
	data, err := os.ReadFile(filePath)
	if err != nil {
		if os.IsNotExist(err) {
			return nil, ErrKeyNotFound
		}
		return nil, fmt.Errorf("failed to read key file: %w", err)
	}

	// Try legacy passphrase
	key, err := crypto.Decrypt(legacyPassphrase, string(data))
	if err != nil {
		// Legacy passphrase didn't work either
		if err == crypto.ErrInvalidPassphrase {
			return nil, ErrNoPassword
		}
		return nil, fmt.Errorf("failed to decrypt master key: %w", err)
	}

	// Legacy decryption worked - return key with migration needed error
	return []byte(key), ErrMigrationNeeded
}

// MigrateToKeyring migrates the master key to use the new keyring-based password system.
// The masterKey should be the decrypted master key, and newPassword is the password to store in keyring.
func (m *Manager) MigrateToKeyring(masterKey []byte, newPassword []byte) error {
	// Store the new password in keyring
	if err := m.SetPassword(newPassword); err != nil {
		return fmt.Errorf("failed to store password in keyring: %w", err)
	}

	// Re-save the master key with the new password
	if err := m.Save(masterKey); err != nil {
		return fmt.Errorf("failed to save master key: %w", err)
	}

	return nil
}

// Delete removes the master key file and the stored password.
func (m *Manager) Delete() error {
	filePath := m.keyFilePath()
	if _, err := os.Stat(filePath); err == nil {
		if err := os.Remove(filePath); err != nil {
			return fmt.Errorf("failed to delete key file: %w", err)
		}
	}

	// Also remove password from keyring
	return m.DeletePassword()
}

// Exists checks if a master key file exists.
func (m *Manager) Exists() (bool, error) {
	filePath := m.keyFilePath()
	_, err := os.Stat(filePath)
	if err == nil {
		return true, nil
	}
	if os.IsNotExist(err) {
		return false, nil
	}
	return false, fmt.Errorf("failed to check key existence: %w", err)
}

// CanLoad attempts to load the master key and returns whether it can be decrypted.
// This is used by init to detect if recovery mode is needed.
func (m *Manager) CanLoad() bool {
	_, err := m.Load(nil)
	return err == nil
}

// RequiresPassword returns true if a password is required to unlock the master key.
// This checks both the keyring (preferred) and environment variable (fallback).
func (m *Manager) RequiresPassword() (bool, error) {
	// Check if password exists in keyring
	hasKeyringPassword, err := m.PasswordExists()
	if err != nil {
		return false, err
	}
	if hasKeyringPassword {
		return true, nil
	}

	// Check if PLY_PASSPHRASE env var is set
	if passphrase := os.Getenv("PLY_PASSPHRASE"); passphrase != "" {
		return true, nil
	}

	return false, nil
}

// PasswordExists returns true if a password is stored in the OS keyring.
func (m *Manager) PasswordExists() (bool, error) {
	_, err := keyring.Get(m.keyringService, m.keyringUser)
	if err == nil {
		return true, nil
	}
	if err == keyring.ErrNotFound {
		return false, nil
	}
	return false, fmt.Errorf("failed to check keyring: %w", err)
}

// GetStoredPassword retrieves the password from the OS keyring.
// This is useful for verifying the password is actually retrievable before
// attempting to use it for encryption.
func (m *Manager) GetStoredPassword() (string, error) {
	return m.getStoredPassword()
}

// SetPassword stores the password in the OS keyring for future use.
// This enables automatic password retrieval on subsequent runs.
func (m *Manager) SetPassword(password []byte) error {
	if len(password) == 0 {
		return ErrInvalidPassword
	}

	err := keyring.Set(m.keyringService, m.keyringUser, string(password))
	if err != nil {
		return fmt.Errorf("failed to store password in keyring: %w", err)
	}

	return nil
}

// DeletePassword removes the password from the OS keyring.
func (m *Manager) DeletePassword() error {
	err := keyring.Delete(m.keyringService, m.keyringUser)
	if err != nil && err != keyring.ErrNotFound {
		return fmt.Errorf("failed to delete password from keyring: %w", err)
	}
	return nil
}

// GenerateKey generates a new random master key for age scrypt-based encryption.
func GenerateKey() ([]byte, error) {
	key := make([]byte, keySize)
	if _, err := rand.Read(key); err != nil {
		return nil, fmt.Errorf("failed to generate key: %w", err)
	}
	return key, nil
}

func (m *Manager) keyFilePath() string {
	return filepath.Join(m.dataDir, keyFileName)
}

// getPassphrase returns the passphrase to use for encryption/decryption.
// Priority: 1. Provided password, 2. Keyring, 3. Environment variable
func (m *Manager) getPassphrase(providedPassword []byte) (string, error) {
	// 1. Use provided password if given
	if len(providedPassword) > 0 {
		return string(providedPassword), nil
	}

	// 2. Try to get from keyring
	passphrase, err := m.getStoredPassword()
	if err == nil {
		return passphrase, nil
	}

	// 3. Fall back to environment variable
	if passphrase := os.Getenv("PLY_PASSPHRASE"); passphrase != "" {
		return passphrase, nil
	}

	// No password available
	return "", ErrNoPassword
}

// getStoredPassword retrieves the password from the OS keyring.
func (m *Manager) getStoredPassword() (string, error) {
	passphrase, err := keyring.Get(m.keyringService, m.keyringUser)
	if err != nil {
		if err == keyring.ErrNotFound {
			return "", ErrNoPassword
		}
		return "", fmt.Errorf("failed to get password from keyring: %w", err)
	}
	return passphrase, nil
}

// Equal securely compares two byte slices in constant time.
func Equal(a, b []byte) bool {
	if len(a) != len(b) {
		return false
	}
	return subtle.ConstantTimeCompare(a, b) == 1
}
