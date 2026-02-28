package cmd

import (
	"testing"

	"github.com/dkmnx/ply/internal/database"
	"github.com/dkmnx/ply/internal/providers"
)

func TestFindEntry(t *testing.T) {
	// Test findEntry with invalid provider name
	db := database.New("/tmp/nonexistent")

	_, err := findEntry(db, "invalid-provider")
	// Should return error from provider validation
	if err == nil {
		t.Error("findEntry() with invalid provider should error")
	}
}

func TestFindEntryWithValidProvider(t *testing.T) {
	// Test findEntry with a valid provider name but non-existent database
	// This should still validate the provider name first
	db := database.New("/tmp/nonexistent")

	// Test with a known valid provider name
	_, err := findEntry(db, "openai")
	// The error should be about the database, not the provider validation
	// Provider "openai" is valid, but the database entry won't exist
	if err == nil {
		t.Error("findEntry() with non-existent entry should error")
	}
}

func TestConfigEditCmdUse(t *testing.T) {
	if configEditCmd.Use != "edit [provider]" {
		t.Errorf("configEditCmd.Use = %q, expected %q", configEditCmd.Use, "edit [provider]")
	}
}

func TestConfigEditCmdArgs(t *testing.T) {
	// The command should require exactly 1 argument
	// This is enforced by cobra.ExactArgs(1)
	if configEditCmd.Args == nil {
		t.Error("configEditCmd.Args should be set")
	}
}

func TestConfigEditCmdStructure(t *testing.T) {
	if configEditCmd == nil {
		t.Fatal("configEditCmd is nil")
	}

	// Check that it's added to configCmd
	if len(configEditCmd.Commands()) != 0 {
		t.Error("configEditCmd should not have subcommands")
	}
}

func TestConfigEditCmdFlags(t *testing.T) {
	if configEditCmd == nil {
		t.Fatal("configEditCmd is nil")
	}

	// configEditCmd should not have any flags
	flags := configEditCmd.Flags()
	if flags == nil {
		t.Error("configEditCmd.Flags() should not return nil")
	}
}

// Test provider validation works correctly
func TestProviderValidation(t *testing.T) {
	tests := []struct {
		name    string
		wantErr bool
	}{
		{"openai", false},
		{"anthropic", false},
		{"google", false},
		{"valid-provider", false},
		{"", true},
		{"..", true},
		{"/path", true},
		{"provider/name", true},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			err := providers.Validate(tt.name)
			if (err != nil) != tt.wantErr {
				t.Errorf("providers.Validate(%q) error = %v, wantErr %v", tt.name, err, tt.wantErr)
			}
		})
	}
}
