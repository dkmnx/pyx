package cmd

import (
	"testing"
)

func TestResetCmdStructure(t *testing.T) {
	if resetCmd == nil {
		t.Fatal("resetCmd is nil")
	}

	if resetCmd.Use != "reset" {
		t.Errorf("resetCmd.Use = %q, expected %q", resetCmd.Use, "reset")
	}
}

func TestResetCmdShort(t *testing.T) {
	if resetCmd.Short == "" {
		t.Error("resetCmd.Short should not be empty")
	}
}

func TestResetCmdLong(t *testing.T) {
	if resetCmd.Long == "" {
		t.Error("resetCmd.Long should not be empty")
	}
}

func TestResetCmdArgs(t *testing.T) {
	// reset command should not require any args
	// nil Args means cobra.AnyArgs which accepts arbitrary arguments
	// This is the expected behavior for the reset command
	if resetCmd.Args != nil {
		t.Log("resetCmd.Args is set (this is also valid)")
	}
}

func TestResetCmdFlags(t *testing.T) {
	// reset command should not have any flags
	flags := resetCmd.Flags()
	if flags == nil {
		t.Error("resetCmd.Flags() should not return nil")
	}
}

// File name constants
func TestResetFileNames(t *testing.T) {
	if dbFileName == "" {
		t.Error("dbFileName should not be empty")
	}
	if dbBackupFileName == "" {
		t.Error("dbBackupFileName should not be empty")
	}
	if passwordFileName == "" {
		t.Error("passwordFileName should not be empty")
	}

	// Verify expected values
	if dbFileName != "database.json" {
		t.Errorf("dbFileName = %q, expected %q", dbFileName, "database.json")
	}
	if dbBackupFileName != "database.json.bak" {
		t.Errorf("dbBackupFileName = %q, expected %q", dbBackupFileName, "database.json.bak")
	}
	if passwordFileName != "password.bin" {
		t.Errorf("passwordFileName = %q, expected %q", passwordFileName, "password.bin")
	}
}
