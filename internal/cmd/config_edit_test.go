package cmd

import (
	"context"
	"io"
	"os"
	"strings"
	"testing"

	"github.com/dkmnx/ply/internal/crypto"
	"github.com/dkmnx/ply/internal/database"
	"github.com/dkmnx/ply/internal/fs"
)

func TestFindEntry_ByProvider(t *testing.T) {
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

	found, err := findEntry(db, "openai")
	if err != nil {
		t.Errorf("findEntry() error = %v", err)
		return
	}
	if found.Provider != "openai" {
		t.Errorf("findEntry() = %q, want %q", found.Provider, "openai")
	}
}

func TestFindEntry_NotFound(t *testing.T) {
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

	_, err := findEntry(db, "non-existent")
	if err == nil {
		t.Error("findEntry() should return error for non-existent entry")
	}
}

func TestFindEntry_InvalidProvider(t *testing.T) {
	tmpDir := t.TempDir()

	db := database.New(tmpDir)
	ctx := context.Background()

	// Load empty database
	if err := db.Load(ctx); err != nil {
		t.Fatalf("Failed to load database: %v", err)
	}

	tests := []struct {
		name        string
		provider    string
		wantErr     bool
		errContains string
	}{
		{"path traversal", "../etc", true, "path traversal"},
		{"invalid characters", "provider@bad", true, "must be 1-50 characters"},
		{"empty string", "", true, "cannot be empty"},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			// Capture stderr to suppress tap.Cancel output
			oldStderr := os.Stderr
			r, w, _ := os.Pipe()
			os.Stderr = w

			entry, err := findEntry(db, tt.provider)

			w.Close()
			os.Stderr = oldStderr
			io.ReadAll(r) // Drain the pipe

			if (err != nil) != tt.wantErr {
				t.Errorf("findEntry() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			if err != nil && tt.errContains != "" {
				if !strings.Contains(err.Error(), tt.errContains) {
					t.Errorf("findEntry() error = %v, should contain %q", err, tt.errContains)
				}
			}
			if entry.Provider != "" {
				t.Errorf("findEntry() returned entry with provider %q, want empty", entry.Provider)
			}
		})
	}
}
