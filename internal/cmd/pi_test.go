package cmd

import (
	"testing"
)

func TestPiCmdStructure(t *testing.T) {
	if piCmd == nil {
		t.Fatal("piCmd is nil")
	}

	// Check that it has subcommands
	if len(piCmd.Commands()) == 0 {
		t.Error("piCmd should have subcommands")
	}

	// Check for install subcommand
	found := false
	for _, cmd := range piCmd.Commands() {
		if cmd.Use == "install" {
			found = true
			break
		}
	}
	if !found {
		t.Error("piCmd should have 'install' subcommand")
	}
}

func TestPiInstallCmdFlags(t *testing.T) {
	if piInstallCmd == nil {
		t.Fatal("piInstallCmd is nil")
	}

	// Check --auto flag
	flag := piInstallCmd.Flags().Lookup("auto")
	if flag == nil {
		t.Error("piInstallCmd should have --auto flag")
	}
}

func TestPiInstallCmdUse(t *testing.T) {
	if piInstallCmd.Use != "install" {
		t.Errorf("piInstallCmd.Use = %q, expected %q", piInstallCmd.Use, "install")
	}
}

func TestPiCmdUse(t *testing.T) {
	if piCmd.Use != "pi" {
		t.Errorf("piCmd.Use = %q, expected %q", piCmd.Use, "pi")
	}
}

func TestAutoDetectPMVariable(t *testing.T) {
	// Test the autoDetectPM variable
	autoDetectPM = true
	if !autoDetectPM {
		t.Error("autoDetectPM should be settable")
	}

	autoDetectPM = false
}

func TestPiCmdShort(t *testing.T) {
	if piCmd.Short == "" {
		t.Error("piCmd.Short should not be empty")
	}
}

func TestPiInstallCmdShort(t *testing.T) {
	if piInstallCmd.Short == "" {
		t.Error("piInstallCmd.Short should not be empty")
	}
}
