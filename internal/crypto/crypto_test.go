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

func TestSecureString(t *testing.T) {
	tests := []struct {
		name  string
		input string
	}{
		{"simple string", "test-api-key"},
		{"empty string", ""},
		{"special characters", "key-with-special-chars-!@#$%"},
		{"unicode", "key-unicode-🔑"},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			ss := NewSecureString(tt.input)

			if got := string(ss.Bytes()); got != tt.input {
				t.Errorf("SecureString.Bytes() = %v, want %v", got, tt.input)
			}

			// Empty strings are considered zeroed
			if tt.input != "" && ss.IsZeroed() {
				t.Error("SecureString.IsZeroed() should be false for non-empty string")
			}
		})
	}
}

func TestSecureStringZero(t *testing.T) {
	ss := NewSecureString("secret-value")

	if ss.IsZeroed() {
		t.Error("SecureString.IsZeroed() should be false before Zero()")
	}

	ss.Zero()

	if !ss.IsZeroed() {
		t.Error("SecureString.IsZeroed() should be true after Zero()")
	}

	if len(ss.Bytes()) != 0 {
		t.Error("SecureString.Bytes() should be empty after Zero()")
	}
}

func TestSecureStringEqual(t *testing.T) {
	ss1 := NewSecureString("same-value")
	ss2 := NewSecureString("same-value")
	ss3 := NewSecureString("different-value")

	tests := []struct {
		name      string
		a         *SecureString
		b         *SecureString
		wantEqual bool
	}{
		{"equal strings", ss1, ss2, true},
		{"different strings", ss1, ss3, false},
		{"nil and nil", nil, nil, true},
		{"nil and non-nil", nil, ss1, false},
		{"non-nil and nil", ss1, nil, false},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			if got := tt.a.Equal(tt.b); got != tt.wantEqual {
				t.Errorf("SecureString.Equal() = %v, want %v", got, tt.wantEqual)
			}
		})
	}
}

func TestSecureStringNil(t *testing.T) {
	var ss *SecureString

	if ss.Bytes() != nil {
		t.Error("SecureString.Bytes() should return nil for nil SecureString")
	}

	if !ss.IsZeroed() {
		t.Error("SecureString.IsZeroed() should return true for nil SecureString")
	}

	ss.Zero() // Should not panic

	other := NewSecureString("test")
	if ss.Equal(other) {
		t.Error("Nil SecureString should not equal non-nil SecureString")
	}

	if other.Equal(ss) {
		t.Error("Non-nil SecureString should not equal nil SecureString")
	}
}
