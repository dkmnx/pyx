package keys

import (
	"os"
	"path/filepath"
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
	defer ZeroMasterKey(masterKey)

	// Step 2: Save master key to storage
	if err := m.Save(masterKey); err != nil {
		if err == ErrPasswordRequired {
			t.Skip("keyring unavailable, skipping integration test")
		}
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
	ZeroMasterKey(loadedKey)

	// Step 4: Encrypt API key with master key
	originalAPIKey := "sk-test-api-key-12345678901234567890"
	cipher, nonce, err := crypto.Encrypt(masterKey, originalAPIKey)
	if err != nil {
		t.Fatalf("Encrypt() error = %v", err)
	}

	// Step 5: Store encrypted key in database
	entry := database.NewEntry("test-provider", cipher, nonce)
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
	decryptedKey, err := crypto.Decrypt(masterKey, loadedEntry.Cipher, loadedEntry.Nonce)
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

// TestIntegration_PasswordBasedEncryption tests the full workflow with password-based encryption.
// This is an integration test that verifies the complete path when keyring is unavailable.
func TestIntegration_PasswordBasedEncryption(t *testing.T) {
	dir := tempDir(t)
	m := New(dir)
	db := database.New(dir)

	// Generate master key
	masterKey, err := GenerateKey()
	if err != nil {
		t.Fatalf("GenerateKey() error = %v", err)
	}
	defer ZeroMasterKey(masterKey)

	// Set a password for encryption
	password := []byte("test-password-123")
	if err := m.SetPassword(password); err != nil {
		t.Fatalf("SetPassword() error = %v", err)
	}
	// Zero password after use
	for i := range password {
		password[i] = 0
	}

	// Save master key with password encryption
	if err := m.Save(masterKey); err != nil {
		t.Fatalf("Save() error = %v", err)
	}

	// Load master key with password
	password = []byte("test-password-123")
	loadedKey, err := m.Load(password)
	// Zero password after use
	for i := range password {
		password[i] = 0
	}
	if err != nil {
		t.Fatalf("Load() error = %v", err)
	}
	defer ZeroMasterKey(loadedKey)

	// Verify keys match
	if !Equal(masterKey, loadedKey) {
		t.Fatal("Loaded key does not match original")
	}

	// Encrypt and decrypt API key
	originalAPIKey := "sk-password-based-test-key"
	cipher, nonce, err := crypto.Encrypt(loadedKey, originalAPIKey)
	if err != nil {
		t.Fatalf("Encrypt() error = %v", err)
	}

	// Store in database
	entry := database.NewEntry("password-provider", cipher, nonce)
	if err := db.AddEntry(entry); err != nil {
		t.Fatalf("db.AddEntry() error = %v", err)
	}

	// Save and load database
	if err := db.Save(t.Context()); err != nil {
		t.Fatalf("db.Save() error = %v", err)
	}
	if err := db.Load(t.Context()); err != nil {
		t.Fatalf("db.Load() error = %v", err)
	}

	loadedEntry, err := db.GetEntry("password-provider")
	if err != nil {
		t.Fatalf("db.GetEntry() error = %v", err)
	}

	// Decrypt and verify
	decryptedKey, err := crypto.Decrypt(loadedKey, loadedEntry.Cipher, loadedEntry.Nonce)
	if err != nil {
		t.Fatalf("Decrypt() error = %v", err)
	}
	defer decryptedKey.Zero()

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
	defer ZeroMasterKey(masterKey)

	// Save master key
	if err := m.Save(masterKey); err != nil {
		if err == ErrPasswordRequired {
			t.Skip("keyring unavailable, skipping integration test")
		}
		t.Fatalf("Save() error = %v", err)
	}

	// Define multiple providers with different API keys
	providers := map[string]string{
		"openai":     "sk-openai-test-key-123",
		"anthropic":  "sk-ant-test-key-456",
		"google":     "google-api-key-789",
		"azure-openai": "azure-test-key-abc",
	}

	// Encrypt and store each provider's API key
	for provider, apiKey := range providers {
		cipher, nonce, err := crypto.Encrypt(masterKey, apiKey)
		if err != nil {
			t.Fatalf("Encrypt() error for %s: %v", provider, err)
		}

		entry := database.NewEntry(provider, cipher, nonce)
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

		decryptedKey, err := crypto.Decrypt(masterKey, entry.Cipher, entry.Nonce)
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

// TestIntegration_MigrationWorkflow tests the legacy key migration workflow
func TestIntegration_MigrationWorkflow(t *testing.T) {
	dir := tempDir(t)
	m := New(dir)
	db := database.New(dir)

	// Create a legacy master key file
	legacyKey := []byte("12345678901234567890123456789012") // Exactly 32 bytes

	// Write legacy key file
	legacyPath := filepath.Join(dir, LegacyKeyFileName)
	if err := os.WriteFile(legacyPath, legacyKey, 0600); err != nil {
		t.Fatalf("Failed to write legacy key: %v", err)
	}

	// Create some encrypted entries with the legacy key
	apiKey := "legacy-migrated-api-key"
	cipher, nonce, err := crypto.Encrypt(legacyKey, apiKey)
	if err != nil {
		t.Fatalf("Encrypt() error = %v", err)
	}

	entry := database.NewEntry("migrated-provider", cipher, nonce)
	if err := db.AddEntry(entry); err != nil {
		t.Fatalf("db.AddEntry() error = %v", err)
	}

	// Save database
	if err := db.Save(t.Context()); err != nil {
		t.Fatalf("db.Save() error = %v", err)
	}

	// Attempt migration
	migrated, err := m.MigrateFromLegacy(db)
	if err != nil {
		t.Fatalf("MigrateFromLegacy() error = %v", err)
	}

	if !migrated {
		t.Fatal("Migration should have occurred")
	}

	// Verify legacy key file is removed
	if _, err := os.Stat(legacyPath); !os.IsNotExist(err) {
		t.Error("Legacy key file should have been removed after migration")
	}

	// Load the migrated master key
	loadedKey, err := m.Load(nil)
	if err != nil {
		t.Fatalf("Load() error = %v", err)
	}
	defer ZeroMasterKey(loadedKey)

	// Verify we can decrypt the database entries with the new key
	if err := db.Load(t.Context()); err != nil {
		t.Fatalf("db.Load() error = %v", err)
	}

	entry, err = db.GetEntry("migrated-provider")
	if err != nil {
		t.Fatalf("db.GetEntry() error = %v", err)
	}

	decryptedKey, err := crypto.Decrypt(loadedKey, entry.Cipher, entry.Nonce)
	if err != nil {
		t.Fatalf("Decrypt() error = %v", err)
	}
	defer decryptedKey.Zero()

	if decryptedKey.String() != apiKey {
		t.Errorf("Decrypted key = %q, want %q", decryptedKey.String(), apiKey)
	}

	// Cleanup
	_ = m.Delete()
}

// ZeroMasterKey securely zeros a master key slice.
// This is a helper for integration tests.
func ZeroMasterKey(key []byte) {
	for i := range key {
		key[i] = 0
	}
}
