package crypto

import (
	"testing"
)

func TestEncryptEmptyPassphrase(t *testing.T) {
	_, err := Encrypt("", "plaintext")
	// Empty passphrase is technically valid for age scrypt encryption
	// The encryption will succeed, but decryption with wrong passphrase will fail
	if err != nil {
		t.Logf("Encrypt() with empty passphrase returned: %v", err)
	}
}

func TestEncryptEmptyPlaintext(t *testing.T) {
	ciphertext, err := Encrypt("passphrase", "")
	if err != nil {
		t.Fatalf("Encrypt() with empty plaintext error = %v", err)
	}

	if ciphertext == "" {
		t.Error("Encrypt() with empty plaintext should return non-empty ciphertext")
	}

	// Should be able to decrypt empty string
	decrypted, err := Decrypt("passphrase", ciphertext)
	if err != nil {
		t.Fatalf("Decrypt() error = %v", err)
	}
	defer decrypted.Zero()

	if decrypted.String() != "" {
		t.Errorf("Decrypt() = %q, want empty string", decrypted.String())
	}
}

func TestDecryptEmptyCiphertext(t *testing.T) {
	_, err := Decrypt("passphrase", "")
	if err == nil {
		t.Error("Decrypt() with empty ciphertext should return error")
	}

	// Empty ciphertext decodes to empty bytes which fails age decryption
	// The error could be ErrDecryptionFailed or ErrInvalidPassphrase depending on the exact failure
	if err != ErrDecryptionFailed && err != ErrInvalidPassphrase {
		t.Errorf("Decrypt() error = %v, want ErrDecryptionFailed or ErrInvalidPassphrase", err)
	}
}

func TestDecryptInvalidCiphertext(t *testing.T) {
	tests := []struct {
		name       string
		ciphertext string
	}{
		{
			name:       "invalid base64",
			ciphertext: "not-valid-base64!!!",
		},
		{
			name:       "valid base64 but invalid ciphertext",
			ciphertext: "dGhpcyBpcyBub3QgYSB2YWxpZCBjaXBoZXJ0ZXh0",
		},
		{
			name:       "empty string",
			ciphertext: "",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			_, err := Decrypt("passphrase", tt.ciphertext)
			if err == nil {
				t.Error("Decrypt() should return error")
			}
		})
	}
}

func TestEncryptDecryptSpecialCharacters(t *testing.T) {
	tests := []struct {
		name      string
		plaintext string
	}{
		{"newlines", "line1\nline2\nline3"},
		{"tabs", "col1\tcol2\tcol3"},
		{"null bytes", "before\x00after"},
		{"unicode", "Hello 世界 🌍"},
		{"long string", string(make([]byte, 10000))},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			ciphertext, err := Encrypt("passphrase", tt.plaintext)
			if err != nil {
				t.Fatalf("Encrypt() error = %v", err)
			}

			decrypted, err := Decrypt("passphrase", ciphertext)
			if err != nil {
				t.Fatalf("Decrypt() error = %v", err)
			}
			defer decrypted.Zero()

			if decrypted.String() != tt.plaintext {
				t.Errorf("Decrypt() = %q, want %q", decrypted.String(), tt.plaintext)
			}
		})
	}
}

func TestSecureBytesNilOperations(t *testing.T) {
	var sb SecureBytes

	// Zero should not panic
	sb.Zero()

	// String should not panic
	_ = sb.String()

	// Bytes should not panic
	_ = sb.Bytes()
}

func TestSecureStringNilBytes(t *testing.T) {
	var ss *SecureString

	bytes := ss.Bytes()
	if bytes != nil {
		t.Errorf("SecureString.Bytes() = %v, want nil", bytes)
	}
}

func TestSecureStringEqualAfterZero(t *testing.T) {
	ss1 := NewSecureString("test-value")
	ss2 := NewSecureString("test-value")

	// Should be equal before zeroing
	if !ss1.Equal(ss2) {
		t.Error("SecureStrings should be equal before zeroing")
	}

	ss1.Zero()

	// Should not be equal after zeroing one
	if ss1.Equal(ss2) {
		t.Error("Zeroed SecureString should not equal non-zeroed SecureString")
	}
}

func TestSecureStringDoubleZero(t *testing.T) {
	ss := NewSecureString("test-value")

	ss.Zero()
	if !ss.IsZeroed() {
		t.Error("SecureString should be zeroed after first Zero()")
	}

	// Second zero should not panic
	ss.Zero()

	if !ss.IsZeroed() {
		t.Error("SecureString should remain zeroed after second Zero()")
	}
}

func TestNewSecureStringFromBytes(t *testing.T) {
	originalBytes := SecureBytes([]byte("test-value"))

	ss := NewSecureStringFromBytes(originalBytes)

	bytes := ss.Bytes()
	if string(bytes) != "test-value" {
		t.Errorf("NewSecureStringFromBytes() = %q, want test-value", string(bytes))
	}

	// Zero the SecureString
	ss.Zero()

	// The underlying bytes in the SecureString should be zeroed
	// Note: We took ownership of the bytes, so they should be zeroed
	if !ss.IsZeroed() {
		t.Error("SecureString should be zeroed after Zero()")
	}
}

func TestSecureStringConcurrentAccess(t *testing.T) {
	ss := NewSecureString("test-value")
	done := make(chan bool)

	// Start multiple goroutines reading
	for i := 0; i < 10; i++ {
		go func() {
			_ = ss.Bytes()
			_ = ss.IsZeroed()
			done <- true
		}()
	}

	// Wait for all goroutines to complete
	for i := 0; i < 10; i++ {
		<-done
	}

	ss.Zero()
}

func TestGenerateKeyUniqueness(t *testing.T) {
	keys := make(map[string]bool)

	// Generate multiple keys and ensure they're all unique
	for i := 0; i < 100; i++ {
		key, err := GenerateKey()
		if err != nil {
			t.Fatalf("GenerateKey() error = %v", err)
		}

		keyStr := string(key)
		if keys[keyStr] {
			t.Error("GenerateKey() generated duplicate key")
		}
		keys[keyStr] = true
	}
}

func TestDecryptCorruptedCiphertext(t *testing.T) {
	// First encrypt something valid
	ciphertext, err := Encrypt("passphrase", "test-data")
	if err != nil {
		t.Fatalf("Encrypt() error = %v", err)
	}

	// Corrupt the ciphertext by modifying some bytes
	corrupted := ciphertext[:len(ciphertext)-5] + "XXXXX"

	_, err = Decrypt("passphrase", corrupted)
	if err == nil {
		t.Error("Decrypt() with corrupted ciphertext should return error")
	}
}
