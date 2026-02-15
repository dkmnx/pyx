// Package crypto provides encryption and decryption utilities for securing API keys.
package crypto

import (
	"crypto/aes"
	"crypto/cipher"
	"crypto/rand"
	"encoding/base64"
	"errors"
	"fmt"
	"io"
)

const (
	keySize   = 32 // 256 bits
	nonceSize = 12 // 96 bits for GCM
)

var (
	ErrInvalidKey        = errors.New("invalid key size: must be 32 bytes")
	ErrInvalidNonce      = errors.New("invalid nonce size: must be 12 bytes")
	ErrInvalidCiphertext = errors.New("invalid ciphertext format")
	ErrZeroed            = errors.New("secure bytes have been zeroed")
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
