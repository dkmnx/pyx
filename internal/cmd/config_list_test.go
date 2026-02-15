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

func TestConfigList_Empty(t *testing.T) {
	tempDir := t.TempDir()

	// Set up environment for test
	origHome := os.Getenv("HOME")
	os.Setenv("HOME", tempDir)
	defer os.Setenv("HOME", origHome)

	// Create empty database
	dataDir, _ := fs.EnsureDataDir()
	db := database.New(dataDir)
	_ = db.Load(context.Background())
	_ = db.Save(context.Background())

	// Create output capture file
	outputFile := tempDir + "/output.txt"
	f, _ := os.Create(outputFile)
	defer f.Close()

	// Create mock command
	cmd := &cobra.Command{}
	cmd.SetOut(f)

	// Run list
	runConfigList(cmd, nil)

	// Read output
	data, _ := os.ReadFile(outputFile)
	output := string(data)

	if output != "No providers configured.\n" {
		t.Errorf("Expected empty list message, got: %s", output)
	}
}

func TestConfigList_NoDatabase(t *testing.T) {
	tempDir := t.TempDir()

	// Set up environment for test
	origHome := os.Getenv("HOME")
	os.Setenv("HOME", tempDir)
	defer os.Setenv("HOME", origHome)

	// Create data directory but no database
	_, _ = fs.EnsureDataDir()

	// Create output capture file
	outputFile := tempDir + "/output.txt"
	f, _ := os.Create(outputFile)
	defer f.Close()

	// Create mock command
	cmd := &cobra.Command{}
	cmd.SetOut(f)

	// Run list
	runConfigList(cmd, nil)

	// Read output
	data, _ := os.ReadFile(outputFile)
	output := string(data)

	if output != "No providers configured.\n" {
		t.Errorf("Expected empty list message, got: %s", output)
	}
}

func TestConfigList_WithEntries(t *testing.T) {
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

	// Create mock command
	cmd := &cobra.Command{}
	cmd.SetOut(f)

	// Run list
	runConfigList(cmd, nil)

	// Read output
	data, _ := os.ReadFile(outputFile)
	output := string(data)

	// Verify header
	if !strings.Contains(output, "Configured providers:") {
		t.Error("Output missing 'Configured providers:' header")
	}

	// Verify all labels are present
	if !strings.Contains(output, "openai-main") {
		t.Error("Output missing 'openai-main' label")
	}
	if !strings.Contains(output, "anthropic-prod") {
		t.Error("Output missing 'anthropic-prod' label")
	}
	if !strings.Contains(output, "google-dev") {
		t.Error("Output missing 'google-dev' label")
	}

	// Verify ID field is present
	if !strings.Contains(output, "ID       : ") {
		t.Error("Output missing 'ID :' field")
	}

	// Verify Provider field is present
	if !strings.Contains(output, "Provider : ") {
		t.Error("Output missing 'Provider :' field")
	}

	// Verify Created field is present
	if !strings.Contains(output, "Created  : ") {
		t.Error("Output missing 'Created :' field")
	}

	// Verify sensitive fields are NOT present
	if strings.Contains(output, "cipher") {
		t.Error("Output should not contain 'cipher' field")
	}
	if strings.Contains(output, "nonce") {
		t.Error("Output should not contain 'nonce' field")
	}

	// Verify bullet point
	if !strings.Contains(output, "❯") {
		t.Error("Output should contain bullet point '❯'")
	}

	// Verify count message
	if !strings.Contains(output, "Total: 3 provider(s)") {
		t.Error("Output missing count message")
	}
}

func TestConfigList_Format(t *testing.T) {
	tempDir := t.TempDir()

	// Set up environment for test
	origHome := os.Getenv("HOME")
	os.Setenv("HOME", tempDir)
	defer os.Setenv("HOME", origHome)

	// Initialize and add single entry
	dataDir, _ := fs.EnsureDataDir()
	masterKey, _ := crypto.GenerateKey()
	_ = fs.SaveMasterKey(masterKey)

	db := database.New(dataDir)
	_ = db.Load(context.Background())

	cipher, nonce, _ := crypto.Encrypt(masterKey, "test-key")
	entry := database.NewEntry("test-label", "test-provider", cipher, nonce)
	_ = db.AddEntry(entry)
	_ = db.Save(context.Background())

	// Create output capture file
	outputFile := tempDir + "/output.txt"
	f, _ := os.Create(outputFile)
	defer f.Close()

	// Create mock command
	cmd := &cobra.Command{}
	cmd.SetOut(f)

	// Run list
	runConfigList(cmd, nil)

	// Read output
	data, _ := os.ReadFile(outputFile)
	output := string(data)

	// Verify format structure
	lines := strings.Split(output, "\n")

	if len(lines) < 5 {
		t.Errorf("Expected at least 5 lines, got %d", len(lines))
	}

	// Line 1 should be header
	if !strings.Contains(lines[0], "Configured providers:") {
		t.Errorf("First line should be header, got: %s", lines[0])
	}

	// Find first entry line (after blank)
	entryLineIndex := -1
	for i := 2; i < len(lines); i++ {
		if strings.Contains(lines[i], "❯") {
			entryLineIndex = i
			break
		}
	}

	if entryLineIndex == -1 {
		t.Fatal("Could not find entry line with bullet point")
	}

	// Verify entry format
	if !strings.Contains(lines[entryLineIndex], "test-label") {
		t.Errorf("Entry line should contain label, got: %s", lines[entryLineIndex])
	}

	// Verify field format (with spacing for alignment)
	expectedFields := []string{"ID       :", "Provider :", "Created  :"}
	for _, field := range expectedFields {
		found := false
		for _, line := range lines {
			if strings.Contains(line, field) {
				found = true
				break
			}
		}
		if !found {
			t.Errorf("Output missing field: %s", field)
		}
	}
}

func TestConfigList_MultipleEntriesFormat(t *testing.T) {
	tempDir := t.TempDir()

	// Set up environment for test
	origHome := os.Getenv("HOME")
	os.Setenv("HOME", tempDir)
	defer os.Setenv("HOME", origHome)

	// Initialize and add multiple entries
	dataDir, _ := fs.EnsureDataDir()
	masterKey, _ := crypto.GenerateKey()
	_ = fs.SaveMasterKey(masterKey)

	db := database.New(dataDir)
	_ = db.Load(context.Background())

	providers := []struct {
		label    string
		provider string
	}{
		{"first", "openai"},
		{"second", "anthropic"},
		{"third", "google"},
	}

	for _, p := range providers {
		cipher, nonce, _ := crypto.Encrypt(masterKey, "key")
		entry := database.NewEntry(p.label, p.provider, cipher, nonce)
		_ = db.AddEntry(entry)
	}
	_ = db.Save(context.Background())

	// Create output capture file
	outputFile := tempDir + "/output.txt"
	f, _ := os.Create(outputFile)
	defer f.Close()

	// Create mock command
	cmd := &cobra.Command{}
	cmd.SetOut(f)

	// Run list
	runConfigList(cmd, nil)

	// Read output
	data, _ := os.ReadFile(outputFile)
	output := string(data)

	// Count bullet points
	bulletCount := strings.Count(output, "❯")
	if bulletCount != 3 {
		t.Errorf("Expected 3 bullet points, got %d", bulletCount)
	}

	// Verify each entry is separated by blank line
	// Check that there are at least 2 blank lines between entries
	blankLineCount := strings.Count(output, "\n\n")
	if blankLineCount < 2 {
		t.Error("Entries should be separated by blank lines")
	}
}
