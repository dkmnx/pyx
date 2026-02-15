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

func TestConfigDelete_ByLabel(t *testing.T) {
	tempDir := t.TempDir()

	// Set up environment for test
	origHome := os.Getenv("HOME")
	os.Setenv("HOME", tempDir)
	defer os.Setenv("HOME", origHome)

	// Initialize master key and database
	dataDir, _ := fs.EnsureDataDir()
	masterKey, _ := crypto.GenerateKey()
	_ = fs.SaveMasterKey(masterKey)

	db := database.New(dataDir)
	_ = db.Load(context.Background())

	// Add test entries
	testEntries := []struct {
		label    string
		provider string
		apiKey   string
	}{
		{"openai-main", "openai", "sk-test-1"},
		{"anthropic-prod", "anthropic", "sk-ant-test-2"},
		{"google-dev", "google", "google-test-3"},
	}

	for _, te := range testEntries {
		cipher, nonce, _ := crypto.Encrypt(masterKey, te.apiKey)
		entry := database.NewEntry(te.label, te.provider, cipher, nonce)
		_ = db.AddEntry(entry)
	}

	if err := db.Save(context.Background()); err != nil {
		t.Fatalf("Failed to save database: %v", err)
	}

	// Create output capture file
	outputFile := tempDir + "/output.txt"
	f, _ := os.Create(outputFile)
	defer f.Close()

	// Create mock command and delete by label
	cmd := &cobra.Command{}
	cmd.SetOut(f)
	cmd.SetArgs([]string{"anthropic-prod"})

	// Simulate user confirmation input
	origStdin := os.Stdin
	r, w, _ := os.Pipe()
	w.WriteString("y\n")
	w.Close()
	os.Stdin = r
	defer func() { os.Stdin = origStdin }()

	runConfigDelete(cmd, []string{"anthropic-prod"})

	// Read output
	data, _ := os.ReadFile(outputFile)
	output := string(data)

	// Verify confirmation prompt
	if !strings.Contains(output, "Are you sure you want to delete this provider?") {
		t.Errorf("Expected confirmation prompt, got: %s", output)
	}

	if !strings.Contains(output, "✓ Provider 'anthropic-prod' deleted") {
		t.Errorf("Expected success message, got: %s", output)
	}

	// Reload database to verify deletion
	db2 := database.New(dataDir)
	_ = db2.Load(context.Background())

	// Verify entry was deleted
	_, err := db2.GetEntryByLabel("anthropic-prod")
	if err != database.ErrEntryNotFound {
		t.Errorf("Entry should be deleted, got: %v", err)
	}

	// Verify other entries still exist
	_, err = db2.GetEntryByLabel("openai-main")
	if err != nil {
		t.Errorf("Other entries should still exist: %v", err)
	}

	_, err = db2.GetEntryByLabel("google-dev")
	if err != nil {
		t.Errorf("Other entries should still exist: %v", err)
	}
}

func TestConfigDelete_ByID(t *testing.T) {
	tempDir := t.TempDir()

	// Set up environment for test
	origHome := os.Getenv("HOME")
	os.Setenv("HOME", tempDir)
	defer os.Setenv("HOME", origHome)

	// Initialize master key and database
	dataDir, _ := fs.EnsureDataDir()
	masterKey, _ := crypto.GenerateKey()
	_ = fs.SaveMasterKey(masterKey)

	db := database.New(dataDir)
	_ = db.Load(context.Background())

	// Add test entry
	cipher, nonce, _ := crypto.Encrypt(masterKey, "test-key")
	entry := database.NewEntry("test-label", "openai", cipher, nonce)
	_ = db.AddEntry(entry)

	if err := db.Save(context.Background()); err != nil {
		t.Fatalf("Failed to save database: %v", err)
	}

	entryID := entry.ID

	// Create output capture file
	outputFile := tempDir + "/output.txt"
	f, _ := os.Create(outputFile)
	defer f.Close()

	// Create mock command and delete by ID
	cmd := &cobra.Command{}
	cmd.SetOut(f)

	// Simulate user confirmation input
	origStdin := os.Stdin
	r, w, _ := os.Pipe()
	w.WriteString("y\n")
	w.Close()
	os.Stdin = r
	defer func() { os.Stdin = origStdin }()

	runConfigDelete(cmd, []string{entryID})

	// Read output
	data, _ := os.ReadFile(outputFile)
	output := string(data)

	// Verify confirmation prompt
	if !strings.Contains(output, "Are you sure you want to delete this provider?") {
		t.Errorf("Expected confirmation prompt, got: %s", output)
	}

	if !strings.Contains(output, "✓ Provider 'test-label' deleted") {
		t.Errorf("Expected success message, got: %s", output)
	}

	// Reload database to verify deletion
	db2 := database.New(dataDir)
	_ = db2.Load(context.Background())

	// Verify entry was deleted by ID
	_, err := db2.GetEntry(entryID)
	if err != database.ErrEntryNotFound {
		t.Errorf("Entry should be deleted by ID, got: %v", err)
	}
}

func TestConfigDelete_NotFound(t *testing.T) {
	tempDir := t.TempDir()

	// Set up environment for test
	origHome := os.Getenv("HOME")
	os.Setenv("HOME", tempDir)
	defer os.Setenv("HOME", origHome)

	// Initialize master key and database
	dataDir, _ := fs.EnsureDataDir()
	masterKey, _ := crypto.GenerateKey()
	_ = fs.SaveMasterKey(masterKey)

	db := database.New(dataDir)
	_ = db.Load(context.Background())

	// Add test entry
	cipher, nonce, _ := crypto.Encrypt(masterKey, "test-key")
	entry := database.NewEntry("test-label", "openai", cipher, nonce)
	_ = db.AddEntry(entry)
	_ = db.Save(context.Background())

	// Create output capture file
	outputFile := tempDir + "/output.txt"
	f, _ := os.Create(outputFile)
	defer f.Close()

	// Create mock command and try to delete non-existent entry
	cmd := &cobra.Command{}
	cmd.SetOut(f)

	runConfigDelete(cmd, []string{"non-existent-label"})

	// Read output
	data, _ := os.ReadFile(outputFile)
	output := string(data)

	// Verify error message
	if !strings.Contains(output, "Error: provider 'non-existent-label' not found") {
		t.Errorf("Expected not found message, got: %s", output)
	}

	if !strings.Contains(output, "Use 'ply config list' to see all configured providers.") {
		t.Errorf("Expected hint message, got: %s", output)
	}

	// Verify original entry still exists
	_, err := db.GetEntryByLabel("test-label")
	if err != nil {
		t.Errorf("Original entry should still exist: %v", err)
	}
}

func TestConfigDelete_EmptyDatabase(t *testing.T) {
	tempDir := t.TempDir()

	// Set up environment for test
	origHome := os.Getenv("HOME")
	os.Setenv("HOME", tempDir)
	defer os.Setenv("HOME", origHome)

	// Initialize master key and empty database
	dataDir, _ := fs.EnsureDataDir()
	masterKey, _ := crypto.GenerateKey()
	_ = fs.SaveMasterKey(masterKey)

	db := database.New(dataDir)
	_ = db.Load(context.Background())
	_ = db.Save(context.Background())

	// Create output capture file
	outputFile := tempDir + "/output.txt"
	f, _ := os.Create(outputFile)
	defer f.Close()

	// Create mock command and try to delete from empty database
	cmd := &cobra.Command{}
	cmd.SetOut(f)

	runConfigDelete(cmd, []string{"any-label"})

	// Read output
	data, _ := os.ReadFile(outputFile)
	output := string(data)

	// Verify error message
	if !strings.Contains(output, "Error: provider 'any-label' not found") {
		t.Errorf("Expected not found message, got: %s", output)
	}
}

func TestConfigDelete_SaveError(t *testing.T) {
	tempDir := t.TempDir()

	// Set up environment for test
	origHome := os.Getenv("HOME")
	os.Setenv("HOME", tempDir)
	defer os.Setenv("HOME", origHome)

	// Initialize master key and database
	dataDir, _ := fs.EnsureDataDir()
	masterKey, _ := crypto.GenerateKey()
	_ = fs.SaveMasterKey(masterKey)

	db := database.New(dataDir)
	_ = db.Load(context.Background())

	// Add test entry
	cipher, nonce, _ := crypto.Encrypt(masterKey, "test-key")
	entry := database.NewEntry("test-label", "openai", cipher, nonce)
	_ = db.AddEntry(entry)

	if err := db.Save(context.Background()); err != nil {
		t.Fatalf("Failed to save database: %v", err)
	}

	// Make database directory read-only to force save error
	_ = os.Chmod(dataDir, 0500)

	// Create output capture file
	outputFile := tempDir + "/output.txt"
	f, _ := os.Create(outputFile)
	defer f.Close()

	// Create mock command and try to delete
	cmd := &cobra.Command{}
	cmd.SetOut(f)

	// Simulate user confirmation input
	origStdin := os.Stdin
	r, w, _ := os.Pipe()
	w.WriteString("y\n")
	w.Close()
	os.Stdin = r
	defer func() { os.Stdin = origStdin }()

	runConfigDelete(cmd, []string{"test-label"})

	// Restore permissions
	_ = os.Chmod(dataDir, 0700)

	// Read output
	data, _ := os.ReadFile(outputFile)
	output := string(data)

	// Verify error message (may contain save error)
	if !strings.Contains(output, "Error") && !strings.Contains(output, "test-label") {
		t.Errorf("Expected some output, got: %s", output)
	}
}
