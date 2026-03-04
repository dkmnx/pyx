package cmd

import (
	"testing"
)

func TestListCmdUse(t *testing.T) {
	if listCmd.Use != "list" {
		t.Errorf("listCmd.Use = %q, expected %q", listCmd.Use, "list")
	}
}

func TestListCmdShort(t *testing.T) {
	if listCmd.Short == "" {
		t.Error("listCmd.Short should not be empty")
	}
}

func TestListCmdLong(t *testing.T) {
	if listCmd.Long == "" {
		t.Error("listCmd.Long should not be empty")
	}
}

func TestListCmdStructure(t *testing.T) {
	if listCmd == nil {
		t.Fatal("listCmd is nil")
	}

	// Check command use
	if listCmd.Use != "list" {
		t.Errorf("listCmd.Use = %q, expected %q", listCmd.Use, "list")
	}

	// Check that it's added to rootCmd
	found := false
	for _, cmd := range rootCmd.Commands() {
		if cmd.Use == "list" {
			found = true
			break
		}
	}
	if !found {
		t.Error("listCmd should be added to rootCmd")
	}
}

func TestListCmdHasNoFlags(t *testing.T) {
	// listCmd should not have any flags
	flags := listCmd.Flags()
	if flags == nil {
		t.Error("listCmd.Flags() should not return nil")
	}
}
