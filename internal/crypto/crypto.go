// Package crypto provides encryption and decryption utilities for securing API keys
// using the age encryption format (filippo.io/age).
package crypto

import (
	"bytes"
	"crypto/rand"
	"crypto/subtle"
	"encoding/base64"
	"errors"
	"io"
	"sync"

	"filippo.io/age"
)

const keySize = 32 // age scrypt identity size (256 bits)

var (
	ErrInvalidPassphrase  = errors.New("invalid passphrase")
	ErrEncryptionFailed   = errors.New("encryption failed")
	ErrDecryptionFailed   = errors.New("decryption failed")
	ErrZeroed             = errors.New("secure bytes have been zeroed")
	ErrSecureStringZeroed = errors.New("secure string has been zeroed")
)

type SecureBytes []byte

func (s *SecureBytes) Zero() {
	for i := range *s {
		(*s)[i] = 0
	}
	*s = nil
}

func (s SecureBytes) String() string {
	return string(s)
}

func (s SecureBytes) Bytes() []byte {
	return []byte(s)
}

// SecureString wraps a SecureBytes to prevent accidental string conversions
// and ensure sensitive data can be securely zeroed. It does not implement
// fmt.Stringer to prevent accidental logging.
type SecureString struct {
	mu    sync.RWMutex
	bytes SecureBytes
}

// NewSecureString creates a SecureString from a regular string.
// The caller is responsible for zeroing the original string if needed.
func NewSecureString(s string) *SecureString {
	return &SecureString{
		bytes: SecureBytes(s),
	}
}

// NewSecureStringFromBytes creates a SecureString from SecureBytes
// without creating an intermediate string. Takes ownership of the
// SecureBytes and will zero it when Zero() is called.
func NewSecureStringFromBytes(sb SecureBytes) *SecureString {
	return &SecureString{
		bytes: sb,
	}
}

// Bytes returns a copy of the underlying bytes. The caller is responsible
// for zeroing the returned slice when done to prevent sensitive data from
// remaining in memory.
func (ss *SecureString) Bytes() []byte {
	if ss == nil {
		return nil
	}
	ss.mu.RLock()
	defer ss.mu.RUnlock()
	if len(ss.bytes) == 0 {
		return nil
	}
	// Return a copy to prevent data races
	result := make([]byte, len(ss.bytes))
	copy(result, ss.bytes)
	return result
}

// Zero securely erases the contents of the SecureString.
// After calling Zero, the SecureString should not be used.
func (ss *SecureString) Zero() {
	if ss == nil {
		return
	}
	ss.mu.Lock()
	defer ss.mu.Unlock()
	ss.bytes.Zero()
}

// IsZeroed returns true if the SecureString has been zeroed.
func (ss *SecureString) IsZeroed() bool {
	if ss == nil {
		return true
	}
	ss.mu.RLock()
	defer ss.mu.RUnlock()
	return len(ss.bytes) == 0
}

// Equal securely compares two SecureString values in constant time.
// Note: When both SecureStrings are non-nil, this method acquires locks in a consistent
// order (ss first, then other) to prevent deadlocks. This is safe even if the same
// SecureString is passed as both arguments (idempotent).
func (ss *SecureString) Equal(other *SecureString) bool {
	if ss == nil && other == nil {
		return true
	}
	if ss == nil || other == nil {
		return false
	}

	ss.mu.RLock()
	defer ss.mu.RUnlock()
	other.mu.RLock()
	defer other.mu.RUnlock()

	if len(ss.bytes) != len(other.bytes) {
		return false
	}

	return subtle.ConstantTimeCompare(ss.bytes, other.bytes) == 1
}

// Encrypt encrypts plaintext using age with the provided passphrase.
// Returns a base64-encoded age ciphertext.
func Encrypt(passphrase string, plaintext string) (string, error) {
	recipient, err := age.NewScryptRecipient(passphrase)
	if err != nil {
		return "", ErrEncryptionFailed
	}

	var buf bytes.Buffer
	w, err := age.Encrypt(&buf, recipient)
	if err != nil {
		return "", ErrEncryptionFailed
	}

	if _, err := io.WriteString(w, plaintext); err != nil {
		return "", ErrEncryptionFailed
	}

	if err := w.Close(); err != nil {
		return "", ErrEncryptionFailed
	}

	return base64.StdEncoding.EncodeToString(buf.Bytes()), nil
}

// Decrypt decrypts a base64-encoded age ciphertext using the provided passphrase.
// Returns SecureBytes that should be zeroed after use via defer.
func Decrypt(passphrase string, ciphertext string) (SecureBytes, error) {
	identity, err := age.NewScryptIdentity(passphrase)
	if err != nil {
		return nil, ErrInvalidPassphrase
	}

	cipherBytes, err := base64.StdEncoding.DecodeString(ciphertext)
	if err != nil {
		return nil, ErrDecryptionFailed
	}

	r, err := age.Decrypt(bytes.NewReader(cipherBytes), identity)
	if err != nil {
		return nil, ErrInvalidPassphrase
	}

	var buf bytes.Buffer
	if _, err := io.Copy(&buf, r); err != nil {
		return nil, ErrDecryptionFailed
	}

	return SecureBytes(buf.Bytes()), nil
}

// GenerateKey generates a new random key for age scrypt-based encryption.
func GenerateKey() ([]byte, error) {
	key := make([]byte, keySize)
	if _, err := rand.Read(key); err != nil {
		return nil, ErrEncryptionFailed
	}
	return key, nil
}
