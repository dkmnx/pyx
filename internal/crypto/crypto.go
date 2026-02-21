// Package crypto provides encryption and decryption utilities for securing API keys.
package crypto

import (
	"crypto/aes"
	"crypto/cipher"
	"crypto/rand"
	"crypto/subtle"
	"encoding/base64"
	"errors"
	"fmt"
	"io"
	"sync"
)

const (
	keySize   = 32 // 256 bits
	nonceSize = 12 // 96 bits for GCM
)

var (
	ErrInvalidKey         = errors.New("invalid key size: must be 32 bytes")
	ErrInvalidNonce       = errors.New("invalid nonce size: must be 12 bytes")
	ErrInvalidCiphertext  = errors.New("invalid ciphertext format")
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

// Encrypt encrypts plaintext using AES-GCM with the provided key.
// Returns base64-encoded ciphertext and nonce.
func Encrypt(key []byte, plaintext string) (string, string, error) {
	if len(key) != keySize {
		return "", "", fmt.Errorf("%w: got %d bytes", ErrInvalidKey, len(key))
	}

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

	ciphertext := gcm.Seal(nil, nonce, []byte(plaintext), nil)

	cipherB64 := base64.StdEncoding.EncodeToString(ciphertext)
	nonceB64 := base64.StdEncoding.EncodeToString(nonce)

	return cipherB64, nonceB64, nil
}

// Decrypt decrypts ciphertext using AES-GCM with the provided key.
// Expects base64-encoded ciphertext and nonce. Returns SecureBytes
// that should be zeroed after use via defer.
func Decrypt(key []byte, cipherB64, nonceB64 string) (SecureBytes, error) {
	if len(key) != keySize {
		return nil, fmt.Errorf("%w: got %d bytes", ErrInvalidKey, len(key))
	}

	ciphertext, err := base64.StdEncoding.DecodeString(cipherB64)
	if err != nil {
		return nil, fmt.Errorf("failed to decode ciphertext: %w", err)
	}

	nonce, err := base64.StdEncoding.DecodeString(nonceB64)
	if err != nil {
		return nil, fmt.Errorf("failed to decode nonce: %w", err)
	}

	if len(nonce) != nonceSize {
		return nil, fmt.Errorf("%w: got %d bytes", ErrInvalidNonce, len(nonce))
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

	return SecureBytes(plaintext), nil
}

// GenerateKey generates a new random 32-byte encryption key.
func GenerateKey() ([]byte, error) {
	key := make([]byte, keySize)
	if _, err := io.ReadFull(rand.Reader, key); err != nil {
		return nil, fmt.Errorf("failed to generate key: %w", err)
	}
	return key, nil
}
