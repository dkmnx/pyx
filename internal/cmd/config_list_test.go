package cmd

import (
	"testing"
)

func TestConfigCmdUse(t *testing.T) {
	if configCmd.Use != "config" {
		t.Errorf("configCmd.Use = %q, expected %q", configCmd.Use, "config")
	}
}

func TestConfigCmdShort(t *testing.T) {
	if configCmd.Short == "" {
		t.Error("configCmd.Short should not be empty")
	}
}

func TestConfigCmdLong(t *testing.T) {
	if configCmd.Long == "" {
		t.Error("configCmd.Long should not be empty")
	}
}

func TestConfigCmdStructure(t *testing.T) {
	if configCmd == nil {
		t.Fatal("configCmd is nil")
	}

	// Check that it's added to rootCmd
	if len(configCmd.Commands()) == 0 {
		t.Error("configCmd should have subcommands")
	}
}

func TestConfigCmdHasSubcommands(t *testing.T) {
	// Check for expected subcommands
	// The subcommands are added via init() functions
	// Note: "edit" is "edit [provider]" and "delete" is "delete [provider]"
	expectedSubcommands := []string{"list", "edit", "delete"}

	for _, sub := range expectedSubcommands {
		found := false
		for _, cmd := range configCmd.Commands() {
			// Check if the command Use starts with the expected subcommand
			if len(cmd.Use) >= len(sub) && cmd.Use[:len(sub)] == sub {
				found = true
				break
			}
		}
		if !found {
			t.Errorf("configCmd should have '%s' subcommand", sub)
		}
	}
}

func TestConfigCmdFlags(t *testing.T) {
	// configCmd should not have any flags directly
	flags := configCmd.Flags()
	if flags == nil {
		t.Error("configCmd.Flags() should not return nil")
	}
}

func TestConfigListCmdUse(t *testing.T) {
	if configListCmd.Use != "list" {
		t.Errorf("configListCmd.Use = %q, expected %q", configListCmd.Use, "list")
	}
}

func TestConfigListCmdShort(t *testing.T) {
	if configListCmd.Short == "" {
		t.Error("configListCmd.Short should not be empty")
	}
}

func TestConfigListCmdStructure(t *testing.T) {
	if configListCmd == nil {
		t.Fatal("configListCmd is nil")
	}

	// Check command use
	if configListCmd.Use != "list" {
		t.Errorf("configListCmd.Use = %q, expected %q", configListCmd.Use, "list")
	}
}
