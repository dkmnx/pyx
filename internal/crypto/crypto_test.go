package crypto

import (
	"testing"
)

func TestEncryptDecrypt(t *testing.T) {
	passphrase := "test-passphrase-123"
	plaintext := "test-api-key-12345"

	ciphertext, err := Encrypt(passphrase, plaintext)
	if err != nil {
		t.Fatalf("Encrypt() error = %v", err)
	}

	if ciphertext == "" {
		t.Error("Encrypt() returned empty ciphertext")
	}

	decrypted, err := Decrypt(passphrase, ciphertext)
	if err != nil {
		t.Fatalf("Decrypt() error = %v", err)
	}
	defer decrypted.Zero()

	if decrypted.String() != plaintext {
		t.Errorf("Decrypt() = %v, want %v", decrypted.String(), plaintext)
	}
}

func TestDecryptWrongPassphrase(t *testing.T) {
	passphrase := "correct-passphrase"
	wrongPassphrase := "wrong-passphrase"

	ciphertext, err := Encrypt(passphrase, "secret-data")
	if err != nil {
		t.Fatalf("Encrypt() error = %v", err)
	}

	_, err = Decrypt(wrongPassphrase, ciphertext)
	if err == nil {
		t.Error("Decrypt() expected error for wrong passphrase")
	}

	if err != ErrInvalidPassphrase {
		t.Errorf("Decrypt() error = %v, want %v", err, ErrInvalidPassphrase)
	}
}

func TestDecryptInvalidBase64(t *testing.T) {
	_, err := Decrypt("passphrase", "not-valid-base64!!!")
	if err == nil {
		t.Error("Decrypt() expected error for invalid base64")
	}

	if err != ErrDecryptionFailed {
		t.Errorf("Decrypt() error = %v, want %v", err, ErrDecryptionFailed)
	}
}

func TestSecureBytesZero(t *testing.T) {
	passphrase := "test-pass"
	ciphertext, _ := Encrypt(passphrase, "secret-data")

	secureBytes, err := Decrypt(passphrase, ciphertext)
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

func TestDifferentPassphrasesProduceDifferentCiphertext(t *testing.T) {
	plaintext := "same-plaintext"
	cipher1, _ := Encrypt("passphrase1", plaintext)
	cipher2, _ := Encrypt("passphrase2", plaintext)

	if cipher1 == cipher2 {
		t.Error("Different passphrases should produce different ciphertexts")
	}
}

func TestSamePassphraseDifferentCiphertext(t *testing.T) {
	plaintext := "same-plaintext"
	cipher1, _ := Encrypt("passphrase", plaintext)
	cipher2, _ := Encrypt("passphrase", plaintext)

	// Age generates a random salt, so same passphrase + same plaintext
	// should produce different ciphertexts
	if cipher1 == cipher2 {
		t.Error("Same passphrase should produce different ciphertexts due to random salt")
	}
}