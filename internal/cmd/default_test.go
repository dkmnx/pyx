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

func TestDefault_ViewCurrent(t *testing.T) {
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

	// Set as default
	_ = fs.SaveDefaultProvider(entry.ID)

	// Create output capture file
	outputFile := tempDir + "/output.txt"
	f, _ := os.Create(outputFile)
	defer f.Close()

	// Create mock command and view default
	cmd := &cobra.Command{}
	cmd.SetOut(f)

	runDefault(cmd, nil)

	// Read output
	data, _ := os.ReadFile(outputFile)
	output := string(data)

	// Verify output contains label
	if !strings.Contains(output, "test-label") {
		t.Errorf("Expected output to contain label, got: %s", output)
	}

	// Verify output contains ID
	if !strings.Contains(output, entry.ID) {
		t.Errorf("Expected output to contain ID, got: %s", output)
	}

	// Verify output contains provider
	if !strings.Contains(output, "openai") {
		t.Errorf("Expected output to contain provider, got: %s", output)
	}
}

func TestDefault_NotSet(t *testing.T) {
	tempDir := t.TempDir()

	// Set up environment for test
	origHome := os.Getenv("HOME")
	os.Setenv("HOME", tempDir)
	defer os.Setenv("HOME", origHome)

	// Create output capture file
	outputFile := tempDir + "/output.txt"
	f, _ := os.Create(outputFile)
	defer f.Close()

	// Create mock command and view default (not set)
	cmd := &cobra.Command{}
	cmd.SetOut(f)

	runDefault(cmd, nil)

	// Read output
	data, _ := os.ReadFile(outputFile)
	output := string(data)

	// Verify message
	if !strings.Contains(output, "No default provider set.") {
		t.Errorf("Expected 'no default' message, got: %s", output)
	}

	if !strings.Contains(output, "Use 'ply default [label|id]' to set a default provider.") {
		t.Errorf("Expected hint message, got: %s", output)
	}
}

func TestDefault_SetByLabel(t *testing.T) {
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
	cipher, nonce, _ := crypto.Encrypt(masterKey, "test-key")
	entry1 := database.NewEntry("test-label-1", "openai", cipher, nonce)
	_ = db.AddEntry(entry1)
	entry2 := database.NewEntry("test-label-2", "anthropic", cipher, nonce)
	_ = db.AddEntry(entry2)
	_ = db.Save(context.Background())

	// Create output capture file
	outputFile := tempDir + "/output.txt"
	f, _ := os.Create(outputFile)
	defer f.Close()

	// Create mock command and set default by label
	cmd := &cobra.Command{}
	cmd.SetOut(f)

	runDefault(cmd, []string{"test-label-2"})

	// Read output
	data, _ := os.ReadFile(outputFile)
	output := string(data)

	// Verify success message
	if !strings.Contains(output, "✓ Default provider set to 'test-label-2'") {
		t.Errorf("Expected success message, got: %s", output)
	}

	// Verify default was saved
	defaultID, _ := fs.LoadDefaultProvider()
	if defaultID != entry2.ID {
		t.Errorf("Default ID = %v, want %v", defaultID, entry2.ID)
	}
}

func TestDefault_SetByID(t *testing.T) {
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

	entryID := entry.ID

	// Create output capture file
	outputFile := tempDir + "/output.txt"
	f, _ := os.Create(outputFile)
	defer f.Close()

	// Create mock command and set default by ID
	cmd := &cobra.Command{}
	cmd.SetOut(f)

	runDefault(cmd, []string{entryID})

	// Read output
	data, _ := os.ReadFile(outputFile)
	output := string(data)

	// Verify success message
	if !strings.Contains(output, "✓ Default provider set to 'test-label'") {
		t.Errorf("Expected success message, got: %s", output)
	}

	// Verify default was saved
	defaultID, _ := fs.LoadDefaultProvider()
	if defaultID != entryID {
		t.Errorf("Default ID = %v, want %v", defaultID, entryID)
	}
}

func TestDefault_NotFound(t *testing.T) {
	tempDir := t.TempDir()

	// Set up environment for test
	origHome := os.Getenv("HOME")
	os.Setenv("HOME", tempDir)
	defer os.Setenv("HOME", origHome)

	// Create output capture file
	outputFile := tempDir + "/output.txt"
	f, _ := os.Create(outputFile)
	defer f.Close()

	// Create mock command and try to set non-existent default
	cmd := &cobra.Command{}
	cmd.SetOut(f)

	runDefault(cmd, []string{"non-existent-label"})

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
}

func TestDefault_DeletedProvider(t *testing.T) {
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

	// Set as default
	_ = fs.SaveDefaultProvider(entry.ID)

	// Delete the entry
	_ = db.DeleteEntry(entry.ID)
	_ = db.Save(context.Background())

	// Create output capture file
	outputFile := tempDir + "/output.txt"
	f, _ := os.Create(outputFile)
	defer f.Close()

	// Create mock command and view default (deleted)
	cmd := &cobra.Command{}
	cmd.SetOut(f)

	runDefault(cmd, nil)

	// Read output
	data, _ := os.ReadFile(outputFile)
	output := string(data)

	// Verify error message
	if !strings.Contains(output, "Error: default provider entry not found") {
		t.Errorf("Expected not found message, got: %s", output)
	}

	if !strings.Contains(output, "The default provider may have been deleted.") {
		t.Errorf("Expected deleted message, got: %s", output)
	}
}
