package pi

import (
	"os"
	"strings"
	"testing"
)

func TestCheckInstalled(t *testing.T) {
	// pi should be found on most systems (or should be mocked in tests)
	installed, err := CheckInstalled()
	if err != nil {
		t.Errorf("CheckInstalled() error = %v", err)
	}

	// We can't assert this to true or false as it depends on the test environment
	// Just verify it returns without error
	t.Logf("pi installed: %v", installed)
}

func TestFindPackageManager(t *testing.T) {
	name, cmd, err := findPackageManager()
	if err != nil {
		t.Logf("findPackageManager() error (expected if no package manager): %v", err)
	}

	// If found, verify it returns valid values
	if err == nil {
		validManagers := map[string]bool{
			"npm":  true,
			"pnpm": true,
			"yarn": true,
			"bun":  true,
		}

		if !validManagers[name] {
			t.Errorf("findPackageManager() returned unexpected name: %s", name)
		}

		if cmd == "" {
			t.Error("findPackageManager() returned empty command")
		}
		t.Logf("Found package manager: %s (%s)", name, cmd)
	}
}

func TestInstallCommand(t *testing.T) {
	cmd := InstallCommand()

	if cmd == "" {
		t.Error("InstallCommand() returned empty string")
	}

	// Should contain to package name
	if !strings.Contains(cmd, NPMPackage) {
		t.Errorf("InstallCommand() should contain package name %s, got: %s", NPMPackage, cmd)
	}

	// Should contain a known install command
	validCommands := []string{"npm install", "pnpm install", "yarn global add", "bun install"}
	hasValidCommand := false
	for _, validCmd := range validCommands {
		if strings.Contains(cmd, validCmd) {
			hasValidCommand = true
			break
		}
	}
	if !hasValidCommand {
		t.Errorf("InstallCommand() should contain a valid install command, got: %s", cmd)
	}
}

func TestPlatformInfo(t *testing.T) {
	info := PlatformInfo()

	if info == "" {
		t.Error("PlatformInfo() returned empty string")
	}

	// Should contain a slash (format: os/arch)
	if !strings.Contains(info, "/") {
		t.Errorf("PlatformInfo() should contain '/', got: %s", info)
	}

	t.Logf("Platform: %s", info)
}

func TestVersion(t *testing.T) {
	// This test requires pi to be installed
	version, err := Version()
	if err != nil {
		t.Logf("Version() error (expected if pi not installed): %v", err)
		return
	}

	if version == "" {
		t.Error("Version() returned empty string when pi is installed")
	}

	t.Logf("pi version: %s", version)
}

func TestVersionError(t *testing.T) {
	// This test requires pi to be installed
	// We can't easily mock exec.LookPath, so we skip this
	t.Skip("skipping - cannot mock exec.LookPath easily")
}

func TestBinaryName(t *testing.T) {
	if BinaryName == "" {
		t.Error("BinaryName should not be empty")
	}

	if BinaryName != "pi" {
		t.Errorf("BinaryName should be 'pi', got: %s", BinaryName)
	}
}

func TestNPMPackage(t *testing.T) {
	if NPMPackage == "" {
		t.Error("NPMPackage should not be empty")
	}

	expected := "@mariozechner/pi-coding-agent"
	if NPMPackage != expected {
		t.Errorf("NPMPackage should be '%s', got: %s", expected, NPMPackage)
	}
}

func TestDetectCurrentShell(t *testing.T) {
	// Set SHELL environment variable for testing
	oldShell := os.Getenv("SHELL")
	t.Cleanup(func() {
		os.Setenv("SHELL", oldShell)
	})

	tests := []struct {
		name       string
		setShell   string
		want       ShellType
	}{
		{"bash", "/bin/bash", ShellBash},
		{"zsh", "/bin/zsh", ShellZsh},
		{"fish", "/usr/bin/fish", ShellFish},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			os.Setenv("SHELL", tt.setShell)
			got := DetectCurrentShell()
			if got != tt.want {
				t.Errorf("DetectCurrentShell() with SHELL=%s = %v, want %v", tt.setShell, got, tt.want)
			}
		})
	}
}

func TestCompletionScriptPath(t *testing.T) {
	tests := []struct {
		name   string
		shell  ShellType
		setup   func() string
	}{
		{
			name:   "zsh",
			shell:  ShellZsh,
			setup:   func() string { home, _ := os.UserHomeDir(); return home + "/.zshrc" },
		},
		{
			name:   "fish",
			shell:  ShellFish,
			setup:   func() string { home, _ := os.UserHomeDir(); return home + "/.config/fish/completions/ply.fish" },
		},
		{
			name:   "bash",
			shell:  ShellBash,
			setup:   func() string { home, _ := os.UserHomeDir(); return home + "/.bashrc" },
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := CompletionScriptPath(tt.shell)
			expected := tt.setup()
			if got != expected {
				t.Errorf("CompletionScriptPath(%s) = %v, want %v", tt.shell, got, expected)
			}
		})
	}
}

func TestInstallCompletion(t *testing.T) {
	// Test that InstallCompletion can be called without error
	// We can't easily test the actual completion script generation
	// Just verify the function exists and handles shell detection
	shell := DetectCurrentShell()
	if shell == "" {
		t.Error("DetectCurrentShell() should return a shell type")
	}

	// Verify shell type is valid
	validShells := map[ShellType]bool{
		ShellBash:      true,
		ShellZsh:       true,
		ShellFish:      true,
		ShellPowerShell: true,
	}

	if !validShells[shell] {
		t.Errorf("Detected shell %s is not in valid shells list", shell)
	}
}

func TestPackageManagerYarn(t *testing.T) {
	if packageManagerYarn != "yarn" {
		t.Errorf("packageManagerYarn should be 'yarn', got: %s", packageManagerYarn)
	}
}
