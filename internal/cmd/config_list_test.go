package cmd

import (
	"encoding/json"
	"os"
	"path/filepath"
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
	_ = db.Load()
	_ = db.Save()

	// Create output capture file
	outputFile := filepath.Join(tempDir, "output.txt")
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
	outputFile := filepath.Join(tempDir, "output.txt")
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
	_ = db.Load()

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

	if err := db.Save(); err != nil {
		t.Fatalf("Failed to save database: %v", err)
	}

	// Create output capture file
	outputFile := filepath.Join(tempDir, "output.txt")
	f, _ := os.Create(outputFile)
	defer f.Close()

	// Create mock command
	cmd := &cobra.Command{}
	cmd.SetOut(f)

	// Run list
	runConfigList(cmd, nil)

	// Read output
	data, _ := os.ReadFile(outputFile)
	outputStr := string(data)

	// Find JSON part (before the "Total:" line)
	totalIndex := indexSubstring(outputStr, "Total:")
	if totalIndex == -1 {
		t.Fatalf("No 'Total:' found in output")
	}

	// Find the start of the "Total:" line
	jsonEnd := -1
	for i := totalIndex - 1; i >= 0; i-- {
		if outputStr[i] == '\n' {
			jsonEnd = i
			break
		}
	}

	if jsonEnd == -1 {
		jsonEnd = 0
	}

	jsonData := []byte(outputStr[:jsonEnd])

	// Verify JSON output
	var entries []listEntry
	if err := json.Unmarshal(jsonData, &entries); err != nil {
		t.Fatalf("Failed to parse JSON output: %v", err)
	}

	if len(entries) != 3 {
		t.Errorf("Expected 3 entries, got %d", len(entries))
	}

	// Verify entry structure
	for _, e := range entries {
		if e.ID == "" {
			t.Error("Entry missing ID")
		}
		if e.Label == "" {
			t.Error("Entry missing label")
		}
		if e.Provider == "" {
			t.Error("Entry missing provider")
		}
		if e.CreatedAt == "" {
			t.Error("Entry missing created_at")
		}
	}

	// Verify count message
	if !contains(outputStr, "Total: 3 provider(s)") {
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
	_ = db.Load()

	cipher, nonce, _ := crypto.Encrypt(masterKey, "test-key")
	entry := database.NewEntry("test-label", "test-provider", cipher, nonce)
	_ = db.AddEntry(entry)
	_ = db.Save()

	// Create output capture file
	outputFile := filepath.Join(dataDir, "output.txt")
	f, _ := os.Create(outputFile)
	defer f.Close()

	// Create mock command
	cmd := &cobra.Command{}
	cmd.SetOut(f)

	// Run list
	runConfigList(cmd, nil)

	// Read output
	data, _ := os.ReadFile(outputFile)
	outputStr := string(data)

	// Find JSON part (before the "Total:" line)
	totalIndex := indexSubstring(outputStr, "Total:")
	if totalIndex == -1 {
		t.Fatalf("No 'Total:' found in output")
	}

	// Find the start of the "Total:" line
	jsonEnd := -1
	for i := totalIndex - 1; i >= 0; i-- {
		if outputStr[i] == '\n' {
			jsonEnd = i
			break
		}
	}

	if jsonEnd == -1 {
		jsonEnd = 0
	}

	jsonData := []byte(outputStr[:jsonEnd])

	// Verify pretty JSON (has indentation)
	if !contains(string(jsonData), "  \"id\"") && !contains(string(jsonData), "\n  {") {
		t.Error("Output should be pretty-printed JSON")
	}

	// Verify required fields are present
	if !contains(string(jsonData), "\"id\"") {
		t.Error("Output missing id field")
	}
	if !contains(string(jsonData), "\"label\"") {
		t.Error("Output missing label field")
	}
	if !contains(string(jsonData), "\"provider\"") {
		t.Error("Output missing provider field")
	}
	if !contains(string(jsonData), "\"created_at\"") {
		t.Error("Output missing created_at field")
	}

	// Verify sensitive fields are NOT present
	if contains(string(jsonData), "\"cipher\"") {
		t.Error("Output should not contain cipher field")
	}
	if contains(string(jsonData), "\"nonce\"") {
		t.Error("Output should not contain nonce field")
	}
}

func indexSubstring(s, substr string) int {
	for i := 0; i <= len(s)-len(substr); i++ {
		if s[i:i+len(substr)] == substr {
			return i
		}
	}
	return -1
}

func contains(s, substr string) bool {
	return len(s) >= len(substr) && (s == substr || len(s) > len(substr) && findSubstring(s, substr))
}

func findSubstring(s, substr string) bool {
	for i := 0; i <= len(s)-len(substr); i++ {
		if s[i:i+len(substr)] == substr {
			return true
		}
	}
	return false
}
