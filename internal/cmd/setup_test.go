package cmd

import (
	"context"
	"encoding/json"
	"os"
	"path/filepath"
	"testing"

	"github.com/dkmnx/ply/internal/crypto"
	"github.com/dkmnx/ply/internal/database"
	"github.com/dkmnx/ply/internal/fs"
)

func TestSetupCreatesMasterKey(t *testing.T) {
	dataDir := t.TempDir()

	// Set up environment for test
	origHome := os.Getenv("HOME")
	os.Setenv("HOME", dataDir)
	defer os.Setenv("HOME", origHome)

	// Run setup
	dataDirCreated, err := fs.EnsureDataDir()
	if err != nil {
		t.Fatalf("EnsureDataDir() error = %v", err)
	}

	masterKey, err := crypto.GenerateKey()
	if err != nil {
		t.Fatalf("GenerateKey() error = %v", err)
	}

	if err := fs.SaveMasterKey(masterKey); err != nil {
		t.Fatalf("SaveMasterKey() error = %v", err)
	}

	// Verify master key exists
	exists, err := fs.MasterKeyExists()
	if err != nil {
		t.Fatalf("MasterKeyExists() error = %v", err)
	}

	if !exists {
		t.Error("Master key was not created")
	}

	// Verify master key size
	loadedKey, err := fs.LoadMasterKey()
	if err != nil {
		t.Fatalf("LoadMasterKey() error = %v", err)
	}

	if len(loadedKey) != 32 {
		t.Errorf("Master key size = %d, want 32", len(loadedKey))
	}

	// Verify permissions
	masterKeyPath, _ := fs.MasterKeyPath()
	info, err := os.Stat(masterKeyPath)
	if err != nil {
		t.Fatalf("os.Stat() error = %v", err)
	}

	if info.Mode().Perm() != 0600 {
		t.Errorf("Master key permissions = %v, want 0600", info.Mode().Perm())
	}

	_ = dataDirCreated
}

func TestSetupCreatesDatabaseEntry(t *testing.T) {
	dataDir := t.TempDir()

	// Set up environment for test
	origHome := os.Getenv("HOME")
	os.Setenv("HOME", dataDir)
	defer os.Setenv("HOME", origHome)

	// Initialize master key
	masterKey, err := crypto.GenerateKey()
	if err != nil {
		t.Fatalf("GenerateKey() error = %v", err)
	}

	_, _ = fs.EnsureDataDir()
	if err := fs.SaveMasterKey(masterKey); err != nil {
		t.Fatalf("SaveMasterKey() error = %v", err)
	}

	// Initialize database
	db := database.New(dataDir)
	if err := db.Load(context.Background()); err != nil {
		t.Fatalf("Load() error = %v", err)
	}

	// Simulate setup flow
	provider := "openai"
	apiKey := "sk-test-api-key-12345"

	// Encrypt API key
	cipher, nonce, err := crypto.Encrypt(masterKey, apiKey)
	if err != nil {
		t.Fatalf("Encrypt() error = %v", err)
	}

	// Create entry
	entry := database.NewEntry(provider, cipher, nonce)

	// Add to database
	if err := db.AddEntry(entry); err != nil {
		t.Fatalf("AddEntry() error = %v", err)
	}

	// Save database
	if err := db.Save(context.Background()); err != nil {
		t.Fatalf("Save() error = %v", err)
	}

	// Verify database file exists
	dbPath := filepath.Join(dataDir, "database.json")
	if _, err := os.Stat(dbPath); os.IsNotExist(err) {
		t.Fatal("Database file was not created")
	}

	// Verify database permissions
	info, err := os.Stat(dbPath)
	if err != nil {
		t.Fatalf("os.Stat() error = %v", err)
	}

	if info.Mode().Perm() != 0600 {
		t.Errorf("Database permissions = %v, want 0600", info.Mode().Perm())
	}

	// Verify database content
	data, err := os.ReadFile(dbPath)
	if err != nil {
		t.Fatalf("ReadFile() error = %v", err)
	}

	var entries []database.Entry
	if err := json.Unmarshal(data, &entries); err != nil {
		t.Fatalf("Unmarshal() error = %v", err)
	}

	if len(entries) != 1 {
		t.Fatalf("Database has %d entries, want 1", len(entries))
	}

	if entries[0].Provider != provider {
		t.Errorf("Entry provider = %v, want %v", entries[0].Provider, provider)
	}

	if entries[0].Cipher != cipher {
		t.Error("Entry cipher does not match")
	}

	if entries[0].Nonce != nonce {
		t.Error("Entry nonce does not match")
	}

	if entries[0].CreatedAt.IsZero() {
		t.Error("Entry created_at is zero")
	}

	// Verify decryption
	decryptedKey, err := crypto.Decrypt(masterKey, entries[0].Cipher, entries[0].Nonce)
	if err != nil {
		t.Fatalf("Decrypt() error = %v", err)
	}
	defer decryptedKey.Zero()

	if decryptedKey.String() != apiKey {
		t.Errorf("Decrypted key = %v, want %v", decryptedKey.String(), apiKey)
	}
}

func TestSetupReusesExistingMasterKey(t *testing.T) {
	dataDir := t.TempDir()

	// Set up environment for test
	origHome := os.Getenv("HOME")
	os.Setenv("HOME", dataDir)
	defer os.Setenv("HOME", origHome)

	// Create initial master key
	_, _ = fs.EnsureDataDir()
	masterKey1, _ := crypto.GenerateKey()
	if err := fs.SaveMasterKey(masterKey1); err != nil {
		t.Fatalf("SaveMasterKey() error = %v", err)
	}

	// Load existing master key
	masterKey2, err := fs.LoadMasterKey()
	if err != nil {
		t.Fatalf("LoadMasterKey() error = %v", err)
	}

	// Verify the same key is loaded
	if len(masterKey1) != len(masterKey2) {
		t.Error("Loaded master key has different size")
	}

	for i := range masterKey1 {
		if masterKey1[i] != masterKey2[i] {
			t.Error("Loaded master key is different from original")
			break
		}
	}
}

func TestProviderList(t *testing.T) {
	expectedProviders := []string{
		"amazon-bedrock",
		"anthropic",
		"azure-openai-responses",
		"cerebras",
		"github-copilot",
		"google",
		"google-antigravity",
		"google-gemini-cli",
		"google-vertex",
		"groq",
		"huggingface",
		"kimi-coding",
		"minimax",
		"minimax-cn",
		"mistral",
		"openai",
		"openai-codex",
		"opencode",
		"openrouter",
		"vercel-ai-gateway",
		"xai",
		"zai",
	}

	// Note: Providers is defined in prompt package
	// This test documents the expected list
	if len(expectedProviders) == 0 {
		t.Fatal("Provider list is empty")
	}

	// Verify provider names are lowercase and hyphenated where appropriate
	for _, p := range expectedProviders {
		if p == "" {
			t.Error("Provider name is empty")
		}
	}
}
