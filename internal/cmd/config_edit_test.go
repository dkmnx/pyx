package cmd

import (
	"context"
	"os"
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
