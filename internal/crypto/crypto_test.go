package crypto

import (
	"errors"
	"testing"
)

func TestEncryptDecrypt(t *testing.T) {
	key := make([]byte, 32)
	for i := range key {
		key[i] = byte(i)
	}

	plaintext := "test-api-key-12345"

	cipher, nonce, err := Encrypt(key, plaintext)
	if err != nil {
		t.Fatalf("Encrypt() error = %v", err)
	}

	if cipher == "" {
		t.Error("Encrypt() returned empty cipher")
	}

	if nonce == "" {
		t.Error("Encrypt() returned empty nonce")
	}

	decrypted, err := Decrypt(key, cipher, nonce)
	if err != nil {
		t.Fatalf("Decrypt() error = %v", err)
	}
	defer decrypted.Zero()

	if decrypted.String() != plaintext {
		t.Errorf("Decrypt() = %v, want %v", decrypted.String(), plaintext)
	}
}

func TestEncryptInvalidKey(t *testing.T) {
	key := make([]byte, 16) // Wrong size

	_, _, err := Encrypt(key, "plaintext")
	if err == nil {
		t.Error("Encrypt() expected error for invalid key size")
	}

	if !errors.Is(err, ErrInvalidKey) {
		t.Errorf("Encrypt() error = %v, want %v", err, ErrInvalidKey)
	}
}

func TestDecryptInvalidKey(t *testing.T) {
	key := make([]byte, 16) // Wrong size

	_, err := Decrypt(key, "ciphertext", "nonce")
	if err == nil {
		t.Error("Decrypt() expected error for invalid key size")
	}

	if !errors.Is(err, ErrInvalidKey) {
		t.Errorf("Decrypt() error = %v, want %v", err, ErrInvalidKey)
	}
}

func TestSecureBytesZero(t *testing.T) {
	key, _ := GenerateKey()
	cipher, nonce, _ := Encrypt(key, "secret-data")

	secureBytes, err := Decrypt(key, cipher, nonce)
	if err != nil {
		t.Fatalf("Decrypt() error = %v", err)
	}

	if len(secureBytes) == 0 {
		t.Fatal("SecureBytes should not be empty")
	}

	secureBytes.Zero()

	if secureBytes != nil {
		t.Error("SecureBytes should be nil after Zero()")
	}
}

func TestGenerateKey(t *testing.T) {
	key1, err := GenerateKey()
	if err != nil {
		t.Fatalf("GenerateKey() error = %v", err)
	}

	if len(key1) != 32 {
		t.Errorf("GenerateKey() key length = %d, want 32", len(key1))
	}

	// Generate another key to ensure they're different
	key2, err := GenerateKey()
	if err != nil {
		t.Fatalf("GenerateKey() error = %v", err)
	}

	// Check that keys are different (not all bytes identical)
	identical := true
	for i := range key1 {
		if key1[i] != key2[i] {
			identical = false
			break
		}
	}

	if identical {
		t.Error("GenerateKey() generated identical keys (statistically impossible)")
	}
}
