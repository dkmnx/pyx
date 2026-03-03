package keys

import (
	"testing"

	"github.com/dkmnx/ply/internal/crypto"
	"github.com/dkmnx/ply/internal/database"
)

// TestIntegration_FullWorkflow tests the complete workflow:
// 1. Generate master key
// 2. Save master key
// 3. Load master key
// 4. Encrypt API key with master key
// 5. Store encrypted key in database
// 6. Load encrypted key from database
// 7. Decrypt API key with master key
// 8. Verify decrypted key matches original
func TestIntegration_FullWorkflow(t *testing.T) {
	dir := tempDir(t)
	m := New(dir)
	db := database.New(dir)

	// Step 1: Generate master key
	masterKey, err := GenerateKey()
	if err != nil {
		t.Fatalf("GenerateKey() error = %v", err)
	}

	// Step 2: Save master key to storage
	if err := m.Save(masterKey); err != nil {
		t.Fatalf("Save() error = %v", err)
	}

	// Step 3: Load master key from storage
	loadedKey, err := m.Load(nil)
	if err != nil {
		t.Fatalf("Load() error = %v", err)
	}

	// Verify loaded key matches original
	if !Equal(masterKey, loadedKey) {
		t.Fatal("Loaded key does not match original")
	}

	// Step 4: Encrypt API key with master key
	originalAPIKey := "sk-test-api-key-12345678901234567890"
	cipher, err := crypto.Encrypt(string(masterKey), originalAPIKey)
	if err != nil {
		t.Fatalf("Encrypt() error = %v", err)
	}

	// Step 5: Store encrypted key in database
	entry := database.NewEntry("test-provider", cipher)
	if err := db.AddEntry(entry); err != nil {
		t.Fatalf("db.AddEntry() error = %v", err)
	}

	// Save database
	if err := db.Save(t.Context()); err != nil {
		t.Fatalf("db.Save() error = %v", err)
	}

	// Step 6: Load database and get encrypted key
	if err := db.Load(t.Context()); err != nil {
		t.Fatalf("db.Load() error = %v", err)
	}

	loadedEntry, err := db.GetEntry("test-provider")
	if err != nil {
		t.Fatalf("db.GetEntry() error = %v", err)
	}

	// Step 7: Decrypt API key with master key
	decryptedKey, err := crypto.Decrypt(string(masterKey), loadedEntry.Cipher)
	if err != nil {
		t.Fatalf("Decrypt() error = %v", err)
	}
	defer decryptedKey.Zero()

	// Step 8: Verify decrypted key matches original
	if decryptedKey.String() != originalAPIKey {
		t.Errorf("Decrypted key = %q, want %q", decryptedKey.String(), originalAPIKey)
	}

	// Cleanup
	_ = m.Delete()
}

// TestIntegration_MultipleProviders tests encryption/decryption with multiple providers
func TestIntegration_MultipleProviders(t *testing.T) {
	dir := tempDir(t)
	m := New(dir)
	db := database.New(dir)

	// Generate master key
	masterKey, err := GenerateKey()
	if err != nil {
		t.Fatalf("GenerateKey() error = %v", err)
	}

	// Save master key
	if err := m.Save(masterKey); err != nil {
		t.Fatalf("Save() error = %v", err)
	}

	// Define multiple providers with different API keys
	providers := map[string]string{
		"openai":    "sk-openai-test-key-123",
		"anthropic": "sk-ant-test-key-456",
		"google":    "google-api-key-789",
	}

	// Encrypt and store each provider's API key
	for provider, apiKey := range providers {
		cipher, err := crypto.Encrypt(string(masterKey), apiKey)
		if err != nil {
			t.Fatalf("Encrypt() error for %s: %v", provider, err)
		}

		entry := database.NewEntry(provider, cipher)
		if err := db.AddEntry(entry); err != nil {
			t.Fatalf("db.AddEntry() error for %s: %v", provider, err)
		}
	}

	// Save database
	if err := db.Save(t.Context()); err != nil {
		t.Fatalf("db.Save() error = %v", err)
	}

	// Load database
	if err := db.Load(t.Context()); err != nil {
		t.Fatalf("db.Load() error = %v", err)
	}

	// Verify each provider's API key can be decrypted
	for provider, expectedAPIKey := range providers {
		entry, err := db.GetEntry(provider)
		if err != nil {
			t.Fatalf("db.GetEntry() error for %s: %v", provider, err)
		}

		decryptedKey, err := crypto.Decrypt(string(masterKey), entry.Cipher)
		if err != nil {
			t.Fatalf("Decrypt() error for %s: %v", provider, err)
		}
		defer decryptedKey.Zero()

		if decryptedKey.String() != expectedAPIKey {
			t.Errorf("%s: decrypted key = %q, want %q", provider, decryptedKey.String(), expectedAPIKey)
		}
	}

	// Cleanup
	_ = m.Delete()
}
