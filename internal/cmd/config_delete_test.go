package cmd

import (
	"context"
	"os"
	"testing"

	"github.com/dkmnx/ply/internal/crypto"
	"github.com/dkmnx/ply/internal/database"
	"github.com/dkmnx/ply/internal/fs"
	"github.com/spf13/cobra"
)

func TestConfigDelete_ByProvider(t *testing.T) {
	tempDir := t.TempDir()

	origHome := os.Getenv("HOME")
	os.Setenv("HOME", tempDir)
	defer os.Setenv("HOME", origHome)

	dataDir, _ := fs.EnsureDataDir()
	masterKey, _ := crypto.GenerateKey()
	_ = fs.SaveMasterKey(masterKey)

	db := database.New(dataDir)
	_ = db.Load(context.Background())

	testEntries := []struct {
		provider string
		apiKey   string
	}{
		{"openai", "sk-test-1"},
		{"anthropic", "sk-ant-test-2"},
		{"google", "google-test-3"},
	}

	for _, te := range testEntries {
		cipher, nonce, _ := crypto.Encrypt(masterKey, te.apiKey)
		entry := database.NewEntry(te.provider, cipher, nonce)
		_ = db.AddEntry(entry)
	}

	if err := db.Save(context.Background()); err != nil {
		t.Fatalf("Failed to save database: %v", err)
	}

	cmd := &cobra.Command{}
	runConfigDelete(cmd, []string{"non-existent"})

	db2 := database.New(dataDir)
	_ = db2.Load(context.Background())

	_, err := db2.GetEntry("openai")
	if err != nil {
		t.Errorf("OpenAI entry should still exist: %v", err)
	}

	_, err = db2.GetEntry("anthropic")
	if err != nil {
		t.Errorf("Anthropic entry should still exist: %v", err)
	}

	_, err = db2.GetEntry("google")
	if err != nil {
		t.Errorf("Google entry should still exist: %v", err)
	}
}

func TestConfigDelete_NotFound(t *testing.T) {
	tempDir := t.TempDir()

	origHome := os.Getenv("HOME")
	os.Setenv("HOME", tempDir)
	defer os.Setenv("HOME", origHome)

	dataDir, _ := fs.EnsureDataDir()
	masterKey, _ := crypto.GenerateKey()
	_ = fs.SaveMasterKey(masterKey)

	db := database.New(dataDir)
	_ = db.Load(context.Background())

	cipher, nonce, _ := crypto.Encrypt(masterKey, "test-key")
	entry := database.NewEntry("openai", cipher, nonce)
	_ = db.AddEntry(entry)
	_ = db.Save(context.Background())

	cmd := &cobra.Command{}
	runConfigDelete(cmd, []string{"non-existent"})

	_, err := db.GetEntry("openai")
	if err != nil {
		t.Errorf("Original entry should still exist: %v", err)
	}
}

func TestConfigDelete_EmptyDatabase(t *testing.T) {
	tempDir := t.TempDir()

	origHome := os.Getenv("HOME")
	os.Setenv("HOME", tempDir)
	defer os.Setenv("HOME", origHome)

	dataDir, _ := fs.EnsureDataDir()
	masterKey, _ := crypto.GenerateKey()
	_ = fs.SaveMasterKey(masterKey)

	db := database.New(dataDir)
	_ = db.Load(context.Background())
	_ = db.Save(context.Background())

	cmd := &cobra.Command{}
	runConfigDelete(cmd, []string{"any"})

	entries := db.ListEntries()
	if len(entries) != 0 {
		t.Errorf("Database should still be empty, got %d entries", len(entries))
	}
}
