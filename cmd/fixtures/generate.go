// Fixture generator for Rust crypto compatibility testing
//
// Usage:
//   go run cmd/fixtures/generate.go <output-dir>
//
// This generates test fixtures that the Rust implementation must be able to decrypt.

package main

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"time"

	"github.com/dkmnx/ply/internal/crypto"
)

type ProviderEntry struct {
	Provider  string    `json:"provider"`
	Cipher    string    `json:"cipher"`
	CreatedAt time.Time `json:"created_at"`
	UpdatedAt time.Time `json:"updated_at"`
}

type Database struct {
	Providers []ProviderEntry `json:"providers"`
}

type ModelsCache struct {
	Version   string            `json:"version"`
	UpdatedAt time.Time         `json:"updated_at"`
	Models    map[string][]string `json:"models"`
}

type Settings struct {
	GitHubSource          *GitHubSource          `json:"github_source,omitempty"`
	CustomProviderEnvVars map[string]string      `json:"customProviderEnvVars"`
}

type GitHubSource struct {
	Owner  string  `json:"owner"`
	Repo   string  `json:"repo"`
	Branch *string `json:"branch,omitempty"`
}

func main() {
	if len(os.Args) < 2 {
		fmt.Fprintf(os.Stderr, "Usage: %s <output-dir>\n", os.Args[0])
		os.Exit(1)
	}

	outputDir := os.Args[1]

	if err := os.MkdirAll(outputDir, 0755); err != nil {
		fmt.Fprintf(os.Stderr, "Failed to create output directory: %v\n", err)
		os.Exit(1)
	}

	// Generate test data
	if err := generateFixtures(outputDir); err != nil {
		fmt.Fprintf(os.Stderr, "Failed to generate fixtures: %v\n", err)
		os.Exit(1)
	}

	fmt.Printf("✓ Generated test fixtures in %s\n", outputDir)
	fmt.Println()
	fmt.Println("Generated files:")
	fmt.Println("  - master.key (encrypted with passphrase 'test-passphrase')")
	fmt.Println("  - database.json (test provider entries)")
	fmt.Println("  - models.json (sample model cache)")
	fmt.Println("  - settings.json (sample settings)")
	fmt.Println("  - README.txt (test instructions)")
}

func generateFixtures(outputDir string) error {
	// Generate master key
	masterKey, err := crypto.GenerateKey()
	if err != nil {
		return fmt.Errorf("failed to generate master key: %w", err)
	}

	// For testing, we'll use a known passphrase
	passphrase := "test-passphrase"

	// Encrypt master key with passphrase
	encryptedMasterKey, err := crypto.Encrypt(passphrase, string(masterKey))
	if err != nil {
		return fmt.Errorf("failed to encrypt master key: %w", err)
	}

	// Write master.key
	masterKeyPath := filepath.Join(outputDir, "master.key")
	if err := os.WriteFile(masterKeyPath, []byte(encryptedMasterKey), 0600); err != nil {
		return fmt.Errorf("failed to write master.key: %w", err)
	}
	fmt.Printf("✓ Generated %s\n", masterKeyPath)

	// Generate database.json with test providers
	database := Database{
		Providers: []ProviderEntry{
			{
				Provider:  "openai",
				Cipher:    encryptWithMasterKey(masterKey, "sk-openai-test-key-12345"),
				CreatedAt: time.Now().UTC(),
				UpdatedAt: time.Now().UTC(),
			},
			{
				Provider:  "anthropic",
				Cipher:    encryptWithMasterKey(masterKey, "sk-ant-test-key-67890"),
				CreatedAt: time.Now().UTC(),
				UpdatedAt: time.Now().UTC(),
			},
		},
	}

	databasePath := filepath.Join(outputDir, "database.json")
	databaseJSON, _ := json.MarshalIndent(database, "", "  ")
	if err := os.WriteFile(databasePath, databaseJSON, 0600); err != nil {
		return fmt.Errorf("failed to write database.json: %w", err)
	}
	fmt.Printf("✓ Generated %s\n", databasePath)

	// Generate models.json
	models := ModelsCache{
		Version: "v1.0.0",
		UpdatedAt: time.Now().UTC(),
		Models: map[string][]string{
			"openai": {
				"openai/gpt-4",
				"openai/gpt-3.5-turbo",
			},
			"anthropic": {
				"anthropic/claude-3-opus",
				"anthropic/claude-3-sonnet",
			},
		},
	}

	modelsPath := filepath.Join(outputDir, "models.json")
	modelsJSON, _ := json.MarshalIndent(models, "", "  ")
	if err := os.WriteFile(modelsPath, modelsJSON, 0600); err != nil {
		return fmt.Errorf("failed to write models.json: %w", err)
	}
	fmt.Printf("✓ Generated %s\n", modelsPath)

	// Generate settings.json
	settings := Settings{
		CustomProviderEnvVars: map[string]string{
			"qwen-cli": "QWEN_CLI_API_KEY",
			"deepseek": "DEEPSEEK_API_KEY",
		},
	}

	settingsPath := filepath.Join(outputDir, "settings.json")
	settingsJSON, _ := json.MarshalIndent(settings, "", "  ")
	if err := os.WriteFile(settingsPath, settingsJSON, 0600); err != nil {
		return fmt.Errorf("failed to write settings.json: %w", err)
	}
	fmt.Printf("✓ Generated %s\n", settingsPath)

	// Generate README with test instructions
	readmePath := filepath.Join(outputDir, "README.txt")
	readme := `Crypto Compatibility Test Fixtures
=====================================

These fixtures are for testing Rust's ability to decrypt Go-generated encrypted data.

Test Data:
----------
- Passphrase: test-passphrase
- Master key: 32 random bytes (encrypted in master.key)
- Providers: openai, anthropic (API keys encrypted with master key)

Testing Instructions:
---------------------
1. Rust implementation should decrypt master.key using passphrase "test-passphrase"
2. Use decrypted master key to decrypt provider ciphers in database.json
3. Verify decrypted API keys match:
   - openai: sk-openai-test-key-12345
   - anthropic: sk-ant-test-key-67890

File Formats:
-------------
All JSON files use the same format as the production application.
The master.key file contains a base64-encoded age ciphertext.

Generated: ` + time.Now().Format(time.RFC3339) + `
`
	if err := os.WriteFile(readmePath, []byte(readme), 0644); err != nil {
		return fmt.Errorf("failed to write README.txt: %w", err)
	}
	fmt.Printf("✓ Generated %s\n", readmePath)

	return nil
}

func encryptWithMasterKey(masterKey []byte, plaintext string) string {
	cipher, err := crypto.EncryptBytes(masterKey, []byte(plaintext))
	if err != nil {
		panic(fmt.Sprintf("Failed to encrypt: %v", err))
	}
	return cipher
}

// Verify decryption works
func verifyDecryption() {
	passphrase := "test-passphrase"
	testCiphertext := encryptWithPassphrase(passphrase, "test-plaintext")
	
	decrypted, err := crypto.Decrypt(passphrase, testCiphertext)
	if err != nil {
		panic(fmt.Sprintf("Decryption verification failed: %v", err))
	}
	defer decrypted.Zero()
	
	if string(decrypted) != "test-plaintext" {
		panic("Decryption verification failed: plaintext mismatch")
	}
}

func encryptWithPassphrase(passphrase, plaintext string) string {
	cipher, err := crypto.Encrypt(passphrase, plaintext)
	if err != nil {
		panic(fmt.Sprintf("Failed to encrypt: %v", err))
	}
	return cipher
}
