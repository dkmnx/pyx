package keys

import (
	"crypto/rand"
	"crypto/subtle"
	"errors"
	"fmt"
	"os"
	"path/filepath"

	"github.com/dkmnx/ply/internal/crypto"
)

const (
	keySize           = 32 // 256 bits
	keyFileName       = "master.key"
	keyringService    = "ply"
	keyringUser       = "master-key"
	defaultPassphrase = "default"
)

var (
	ErrKeyNotFound     = errors.New("master key not found")
	ErrInvalidKeyData  = errors.New("invalid key data")
	ErrInvalidPassword = errors.New("invalid password")
)

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

// Save saves the master key to a file, encrypted with age.
func (m *Manager) Save(key []byte) error {
	passphrase := getPassphrase()
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

// Load loads the master key from the file.
func (m *Manager) Load(_ []byte) ([]byte, error) {
	passphrase := getPassphrase()

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
		return nil, fmt.Errorf("failed to decrypt master key: %w", err)
	}

	return []byte(key), nil
}

// Delete removes the master key file.
func (m *Manager) Delete() error {
	filePath := m.keyFilePath()
	if _, err := os.Stat(filePath); err == nil {
		if err := os.Remove(filePath); err != nil {
			return fmt.Errorf("failed to delete key file: %w", err)
		}
	}
	return nil
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

// RequiresPassword returns false (no password required, just master.key file).
func (m *Manager) RequiresPassword() (bool, error) {
	return false, nil
}

// PasswordExists returns false.
func (m *Manager) PasswordExists() (bool, error) {
	return false, nil
}

// SetPassword is a no-op for file-based storage.
func (m *Manager) SetPassword(_ []byte) error {
	return nil
}

// GenerateKey generates a new random master key.
func GenerateKey() ([]byte, error) {
	key := make([]byte, keySize)
	if _, err := rand.Read(key); err != nil {
		return nil, fmt.Errorf("failed to generate key: %w", err)
	}
	return key, nil
}

// keyFilePath returns the path to the key file.
func (m *Manager) keyFilePath() string {
	return filepath.Join(m.dataDir, keyFileName)
}

// getPassphrase returns the passphrase from env or default.
func getPassphrase() string {
	if passphrase := os.Getenv("PLY_PASSPHRASE"); passphrase != "" {
		return passphrase
	}
	return defaultPassphrase
}

// Equal securely compares two byte slices in constant time.
func Equal(a, b []byte) bool {
	if len(a) != len(b) {
		return false
	}
	return subtle.ConstantTimeCompare(a, b) == 1
}
