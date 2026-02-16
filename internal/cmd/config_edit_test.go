package cmd

import (
	"context"
	"os"
	"strings"
	"testing"

	"github.com/dkmnx/ply/internal/crypto"
	"github.com/dkmnx/ply/internal/database"
	"github.com/dkmnx/ply/internal/fs"
	"github.com/spf13/cobra"
)

func TestFindEntry_ByLabel(t *testing.T) {
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
	entry := database.NewEntry("test-label", "openai", cipher, nonce)
	_ = db.AddEntry(entry)
	_ = db.Save(context.Background())

	cmd := &cobra.Command{}
	found, err := findEntry(cmd, db, "test-label")
	if err != nil {
		t.Errorf("findEntry() error = %v", err)
		return
	}
	if found.Label != "test-label" {
		t.Errorf("findEntry() = %q, want %q", found.Label, "test-label")
	}
}

func TestFindEntry_ByID(t *testing.T) {
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
	entry := database.NewEntry("test-label", "openai", cipher, nonce)
	_ = db.AddEntry(entry)
	_ = db.Save(context.Background())

	cmd := &cobra.Command{}
	found, err := findEntry(cmd, db, entry.ID)
	if err != nil {
		t.Errorf("findEntry() error = %v", err)
		return
	}
	if found.ID != entry.ID {
		t.Errorf("findEntry() = %q, want %q", found.ID, entry.ID)
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
	entry := database.NewEntry("test-label", "openai", cipher, nonce)
	_ = db.AddEntry(entry)
	_ = db.Save(context.Background())

	outputFile := tempDir + "/output.txt"
	f, _ := os.Create(outputFile)
	defer f.Close()

	cmd := &cobra.Command{}
	cmd.SetOut(f)

	_, err := findEntry(cmd, db, "non-existent")
	if err == nil {
		t.Error("findEntry() should return error for non-existent entry")
	}

	data, _ := os.ReadFile(outputFile)
	output := string(data)
	if !strings.Contains(output, "not found") {
		t.Errorf("Expected 'not found' message, got: %s", output)
	}
}

func TestHandleMetadataUpdate_LabelChange(t *testing.T) {
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
	entry := database.NewEntry("old-label", "openai", cipher, nonce)
	_ = db.AddEntry(entry)
	_ = db.Save(context.Background())

	outputFile := tempDir + "/output.txt"
	f, _ := os.Create(outputFile)
	defer f.Close()

	cmd := &cobra.Command{}
	cmd.SetOut(f)

	handleMetadataUpdate(cmd, db, &entry, "new-label", "openai", "")
	if entry.Label != "new-label" {
		t.Errorf("handleMetadataUpdate() label = %q, want %q", entry.Label, "new-label")
	}

	data, _ := os.ReadFile(outputFile)
	output := string(data)
	if !strings.Contains(output, "✓ Provider 'new-label' updated") {
		t.Errorf("Expected success message, got: %s", output)
	}
}

func TestHandleMetadataUpdate_NoChange(t *testing.T) {
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
	entry := database.NewEntry("test-label", "openai", cipher, nonce)
	_ = db.AddEntry(entry)
	_ = db.Save(context.Background())

	outputFile := tempDir + "/output.txt"
	f, _ := os.Create(outputFile)
	defer f.Close()

	cmd := &cobra.Command{}
	cmd.SetOut(f)

	handleMetadataUpdate(cmd, db, &entry, "test-label", "openai", "")

	data, _ := os.ReadFile(outputFile)
	output := string(data)
	if strings.Contains(output, "✓ Provider") {
		t.Errorf("Should not print success message when no change, got: %s", output)
	}
}

func TestHandleMetadataUpdate_DuplicateLabel(t *testing.T) {
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
	entry1 := database.NewEntry("label-1", "openai", cipher, nonce)
	entry2 := database.NewEntry("label-2", "anthropic", cipher, nonce)
	_ = db.AddEntry(entry1)
	_ = db.AddEntry(entry2)
	_ = db.Save(context.Background())

	outputFile := tempDir + "/output.txt"
	f, _ := os.Create(outputFile)
	defer f.Close()

	cmd := &cobra.Command{}
	cmd.SetOut(f)

	entry1.Label = "label-2"
	handleMetadataUpdate(cmd, db, &entry1, "label-2", "anthropic", "")

	data, _ := os.ReadFile(outputFile)
	output := string(data)
	if !strings.Contains(output, "already exists") {
		t.Errorf("Expected duplicate label error, got: %s", output)
	}
}
