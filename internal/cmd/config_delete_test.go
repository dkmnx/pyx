package cmd

import (
	"context"
	"io"
	"os"
	"path/filepath"
	"testing"

	"github.com/dkmnx/ply/internal/crypto"
	"github.com/dkmnx/ply/internal/database"
	"github.com/dkmnx/ply/internal/fs"
	"github.com/dkmnx/ply/internal/prompt"
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
		cipher, _ := crypto.Encrypt(string(masterKey), te.apiKey)
		entry := database.NewEntry(te.provider, cipher)
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

	cipher, _ := crypto.Encrypt(string(masterKey), "test-key")
	entry := database.NewEntry("openai", cipher)
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
	// Use a subdirectory in temp dir to avoid any existing files
	tempDir := t.TempDir()
	plyDataDir := filepath.Join(tempDir, "ply-data")

	// Backup and set home directory
	origHome := os.Getenv("HOME")
	os.Setenv("HOME", tempDir)
	defer func() {
		os.Setenv("HOME", origHome)
	}()

	// Mock prompt to auto-confirm
	prompt.SetConfirmForTesting(true)
	defer prompt.ResetConfirmForTesting()

	// Create fresh database in a clean subdirectory
	if err := os.MkdirAll(plyDataDir, 0700); err != nil {
		t.Fatalf("MkdirAll() error = %v", err)
	}

	db := database.New(plyDataDir)

	// Load should be safe on empty database
	if err := db.Load(context.Background()); err != nil {
		t.Fatalf("Load() error = %v", err)
	}

	// Ensure database is actually empty
	entries := db.ListEntries()
	if len(entries) != 0 {
		t.Fatalf("Database should be empty at start, got %d entries: %+v", len(entries), entries)
	}

	cmd := &cobra.Command{}
	// Use a valid provider name to avoid validation error
	runConfigDelete(cmd, []string{"openai"})

	// Check database is still empty after delete attempt
	entries = db.ListEntries()
	if len(entries) != 0 {
		t.Errorf("Database should still be empty, got %d entries", len(entries))
	}
}

func TestConfigDelete_InvalidProvider(t *testing.T) {
	tests := []struct {
		name     string
		provider string
	}{
		{"path traversal", "../etc"},
		{"invalid characters", "provider@bad"},
		{"empty string", ""},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			tempDir := t.TempDir()

			origHome := os.Getenv("HOME")
			os.Setenv("HOME", tempDir)
			defer os.Setenv("HOME", origHome)

			dataDir, _ := fs.EnsureDataDir()
			masterKey, _ := crypto.GenerateKey()
			_ = fs.SaveMasterKey(masterKey)

			db := database.New(dataDir)
			_ = db.Load(context.Background())

			// Add a test provider
			cipher, _ := crypto.Encrypt(string(masterKey), "test-key")
			entry := database.NewEntry("openai", cipher)
			_ = db.AddEntry(entry)
			_ = db.Save(context.Background())

			// Capture stderr to suppress tap.Cancel output
			oldStderr := os.Stderr
			r, w, _ := os.Pipe()
			os.Stderr = w

			// Run delete with invalid provider
			cmd := &cobra.Command{}
			runConfigDelete(cmd, []string{tt.provider})

			_ = w.Close()
			os.Stderr = oldStderr
			io.ReadAll(r) // Drain the pipe
			_ = r.Close()

			// Verify the original entry still exists (validation prevented deletion)
			db2 := database.New(dataDir)
			_ = db2.Load(context.Background())

			_, err := db2.GetEntry("openai")
			if err != nil {
				t.Errorf("Original entry should still exist after invalid provider attempt: %v", err)
			}
		})
	}
}
