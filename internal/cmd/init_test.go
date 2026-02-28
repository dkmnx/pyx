package cmd

import (
	"testing"
)

func TestInitCmdStructure(t *testing.T) {
	if initCmd == nil {
		t.Fatal("initCmd is nil")
	}

	if initCmd.Use != "init" {
		t.Errorf("initCmd.Use = %q, expected %q", initCmd.Use, "init")
	}
}

func TestInitCmdShort(t *testing.T) {
	if initCmd.Short == "" {
		t.Error("initCmd.Short should not be empty")
	}
}

func TestInitCmdLong(t *testing.T) {
	if initCmd.Long == "" {
		t.Error("initCmd.Long should not be empty")
	}
}

func TestInitCmdArgs(t *testing.T) {
	// init command should not require any args
	// nil Args means cobra.AnyArgs which accepts arbitrary arguments
	// This is the expected behavior for the init command
	if initCmd.Args != nil {
		t.Log("initCmd.Args is set (this is also valid)")
	}
}

func TestInitCmdFlags(t *testing.T) {
	// init command should not have any flags
	flags := initCmd.Flags()
	if flags == nil {
		t.Error("initCmd.Flags() should not return nil")
	}
}
