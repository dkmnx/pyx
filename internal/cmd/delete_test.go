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

func TestDelete_ByProvider(t *testing.T) {
	tempDir := t.TempDir()

	origHome := os.Getenv("HOME")
	origXdgDataHome := os.Getenv("XDG_DATA_HOME")
	os.Setenv("HOME", tempDir)
	os.Setenv("XDG_DATA_HOME", filepath.Join(tempDir, ".local", "share"))
	defer func() {
		os.Setenv("HOME", origHome)
		os.Setenv("XDG_DATA_HOME", origXdgDataHome)
	}()

	dataDir, err := fs.EnsureDataDir()
	if err != nil {
		t.Fatalf("Failed to create data dir: %v", err)
	}
	masterKey, err := crypto.GenerateKey()
	if err != nil {
		t.Fatalf("Failed to generate master key: %v", err)
	}
	if err := fs.SaveMasterKey(masterKey); err != nil {
		t.Fatalf("Failed to save master key: %v", err)
	}

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
	runDelete(cmd, []string{"non-existent"})

	db2 := database.New(dataDir)
	_ = db2.Load(context.Background())

	_, err = db2.GetEntry("openai")
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

func TestDelete_NotFound(t *testing.T) {
	tempDir := t.TempDir()

	origHome := os.Getenv("HOME")
	origXdgDataHome := os.Getenv("XDG_DATA_HOME")
	os.Setenv("HOME", tempDir)
	os.Setenv("XDG_DATA_HOME", filepath.Join(tempDir, ".local", "share"))
	defer func() {
		os.Setenv("HOME", origHome)
		os.Setenv("XDG_DATA_HOME", origXdgDataHome)
	}()

	dataDir, err := fs.EnsureDataDir()
	if err != nil {
		t.Fatalf("Failed to create data dir: %v", err)
	}
	masterKey, err := crypto.GenerateKey()
	if err != nil {
		t.Fatalf("Failed to generate master key: %v", err)
	}
	if err := fs.SaveMasterKey(masterKey); err != nil {
		t.Fatalf("Failed to save master key: %v", err)
	}

	db := database.New(dataDir)
	_ = db.Load(context.Background())

	cipher, _ := crypto.Encrypt(string(masterKey), "test-key")
	entry := database.NewEntry("openai", cipher)
	_ = db.AddEntry(entry)
	_ = db.Save(context.Background())

	cmd := &cobra.Command{}
	runDelete(cmd, []string{"non-existent"})

	_, err = db.GetEntry("openai")
	if err != nil {
		t.Errorf("Original entry should still exist: %v", err)
	}
}

func TestDelete_EmptyDatabase(t *testing.T) {
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
	runDelete(cmd, []string{"openai"})

	// Check database is still empty after delete attempt
	entries = db.ListEntries()
	if len(entries) != 0 {
		t.Errorf("Database should still be empty, got %d entries", len(entries))
	}
}

func TestDelete_InvalidProvider(t *testing.T) {
	tests := []struct {
		name     string
		provider string
	}{
		{"path_traversal", "../etc"},
		{"invalid_characters", "provider@bad"},
		{"empty_string", ""},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			tempDir := t.TempDir()

			origHome := os.Getenv("HOME")
			origXdgDataHome := os.Getenv("XDG_DATA_HOME")
			os.Setenv("HOME", tempDir)
			os.Setenv("XDG_DATA_HOME", filepath.Join(tempDir, ".local", "share"))
			defer func() {
				os.Setenv("HOME", origHome)
				os.Setenv("XDG_DATA_HOME", origXdgDataHome)
			}()

			dataDir, err := fs.EnsureDataDir()
			if err != nil {
				t.Fatalf("Failed to create data dir: %v", err)
			}
			masterKey, err := crypto.GenerateKey()
			if err != nil {
				t.Fatalf("Failed to generate master key: %v", err)
			}
			if err := fs.SaveMasterKey(masterKey); err != nil {
				t.Fatalf("Failed to save master key: %v", err)
			}

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
			runDelete(cmd, []string{tt.provider})

			_ = w.Close()
			os.Stderr = oldStderr
			io.ReadAll(r) // Drain the pipe
			_ = r.Close()

			// Verify the original entry still exists (validation prevented deletion)
			db2 := database.New(dataDir)
			_ = db2.Load(context.Background())

			_, err = db2.GetEntry("openai")
			if err != nil {
				t.Errorf("Original entry should still exist after invalid delete attempt: %v", err)
			}
		})
	}
}

func TestDeleteCmdStructure(t *testing.T) {
	if deleteCmd == nil {
		t.Fatal("deleteCmd is nil")
	}

	if deleteCmd.Use != "delete [provider]" {
		t.Errorf("deleteCmd.Use = %q, expected %q", deleteCmd.Use, "delete [provider]")
	}

	// Check that it's added to rootCmd
	found := false
	for _, cmd := range rootCmd.Commands() {
		if cmd.Use == "delete [provider]" {
			found = true
			break
		}
	}
	if !found {
		t.Error("deleteCmd should be added to rootCmd")
	}
}