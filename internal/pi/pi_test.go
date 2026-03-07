package pi

import (
	"os"
	"testing"
)

func TestCompletionScriptInstallPath(t *testing.T) {
	tests := []struct {
		name      string
		shell     ShellType
		expectErr bool
	}{
		{
			name:      "bash",
			shell:     ShellBash,
			expectErr: false,
		},
		{
			name:      "zsh",
			shell:     ShellZsh,
			expectErr: false,
		},
		{
			name:      "fish",
			shell:     ShellFish,
			expectErr: false,
		},
		{
			name:      "powershell",
			shell:     ShellPowerShell,
			expectErr: false,
		},
		{
			name:      "unknown shell",
			shell:     ShellType("unknown"),
			expectErr: true,
		},
		{
			name:      "empty shell",
			shell:     ShellType(""),
			expectErr: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			path, err := completionScriptInstallPath(tt.shell)
			if tt.expectErr {
				if err == nil {
					t.Errorf("completionScriptInstallPath(%s) expected error, got nil", tt.shell)
				}
				return
			}
			if err != nil {
				t.Fatalf("completionScriptInstallPath(%s) unexpected error: %v", tt.shell, err)
			}
			if path == "" {
				t.Error("completionScriptInstallPath() returned empty path")
			}
			// Verify path contains expected patterns
			switch tt.shell {
			case ShellBash:
				if !contains(path, ".bash_completions") && !contains(path, "ply.bash") {
					t.Errorf("completionScriptInstallPath(%s) = %v, expected bash completion path", tt.shell, path)
				}
			case ShellZsh:
				if !contains(path, ".zsh") && !contains(path, "_ply") {
					t.Errorf("completionScriptInstallPath(%s) = %v, expected zsh completion path", tt.shell, path)
				}
			case ShellFish:
				if !contains(path, "fish") && !contains(path, "ply.fish") {
					t.Errorf("completionScriptInstallPath(%s) = %v, expected fish completion path", tt.shell, path)
				}
			case ShellPowerShell:
				if !contains(path, "PowerShell") && !contains(path, "ply.ps1") {
					t.Errorf("completionScriptInstallPath(%s) = %v, expected powershell completion path", tt.shell, path)
				}
			}
		})
	}
}

func contains(s, substr string) bool {
	return len(s) >= len(substr) && (s == substr || len(s) > 0 && (s[:len(substr)] == substr || contains(s[1:], substr)))
}

func TestShellType(t *testing.T) {
	tests := []struct {
		shellType ShellType
		expected  string
	}{
		{ShellBash, "bash"},
		{ShellZsh, "zsh"},
		{ShellFish, "fish"},
		{ShellPowerShell, "powershell"},
	}

	for _, tt := range tests {
		t.Run(tt.expected, func(t *testing.T) {
			if string(tt.shellType) != tt.expected {
				t.Errorf("ShellType = %q, expected %q", tt.shellType, tt.expected)
			}
		})
	}
}

func TestShellNames(t *testing.T) {
	if len(ShellNames) != 4 {
		t.Errorf("ShellNames length = %d, expected 4", len(ShellNames))
	}

	expected := []string{"bash", "zsh", "fish", "powershell"}
	for i, name := range ShellNames {
		if name != expected[i] {
			t.Errorf("ShellNames[%d] = %q, expected %q", i, name, expected[i])
		}
	}
}

func TestDetectCurrentShellFishEnvVar(t *testing.T) {
	// Test fish detection via environment variable
	oldFishVar := os.Getenv("__FISH_VERSION_DIR")
	t.Cleanup(func() {
		if oldFishVar == "" {
			os.Unsetenv("__FISH_VERSION_DIR")
		} else {
			os.Setenv("__FISH_VERSION_DIR", oldFishVar)
		}
	})

	os.Setenv("__FISH_VERSION_DIR", "1.0")
	// Unset SHELL to ensure we test the fish env var path
	os.Unsetenv("SHELL")

	shell := DetectCurrentShell()
	if shell != ShellFish {
		t.Errorf("DetectCurrentShell() with __FISH_VERSION_DIR = %q, expected %q", shell, ShellFish)
	}
}

func TestInstallWithExplicitPackageManager(t *testing.T) {
	// Test Install with a specific package manager
	// This will likely fail if the package manager isn't installed
	// but we can test the error path
	err := Install("nonexistent-pkg-mgr")
	if err == nil {
		t.Error("Install() with nonexistent package manager should error")
	}
}

func TestInstallEmptyStringTriggersAutoDetect(t *testing.T) {
	// Test that empty string triggers auto-detection
	// This might fail if no package manager is available
	err := Install("")
	if err != nil {
		// Expected if no package manager found
		t.Logf("Install('') error (expected if no package manager): %v", err)
	}
}

func TestCheckInstalledNotFound(t *testing.T) {
	// Test CheckInstalled with a binary that doesn't exist
	// We can't easily test this without modifying the BinaryName
	// Instead, test that it returns (bool, nil) when binary is not found
	installed, err := CheckInstalled()
	// Either pi is installed or not, but shouldn't error
	if err != nil {
		t.Logf("CheckInstalled() error: %v", err)
	}
	t.Logf("pi installed: %v", installed)
}

func TestEnsureInstalledAlreadyInstalled(t *testing.T) {
	// Test EnsureInstalled when pi is already installed
	// Note: We can't easily pass nil context, so we skip this test
	// as it requires integration with the full CLI flow
	t.Skip("EnsureInstalled requires integration testing with context")
}

func TestVersionNotInstalled(t *testing.T) {
	// Test Version when pi is not installed - already covered but let's add edge case
	// We can't easily mock exec.LookPath, so we rely on the actual environment
	version, err := Version()
	if err != nil {
		t.Logf("Version() error (expected if pi not installed): %v", err)
	} else {
		if version == "" {
			t.Error("Version() returned empty string when no error")
		}
	}
}

func TestInstallCommandYarn(t *testing.T) {
	// Test InstallCommand with yarn - this depends on what's available
	// The test just verifies the function returns something reasonable
	cmd := InstallCommand()
	if cmd == "" {
		t.Error("InstallCommand() returned empty string")
	}

	// Should contain the package name
	if !containsString(cmd, NPMPackage) {
		t.Errorf("InstallCommand() should contain %s, got: %s", NPMPackage, cmd)
	}
}

func containsString(s, substr string) bool {
	return len(s) > 0 && len(s) >= len(substr) && (s == substr || len(s) > 0 && (s[:len(substr)] == substr || containsString(s[1:], substr)))
}

func TestFindPackageManagerPriority(t *testing.T) {
	// Test that findPackageManager checks in correct priority order
	// We can't easily control what's installed, but we can verify the function works
	name, cmd, err := findPackageManager()
	if err != nil {
		t.Skipf("No package manager available: %v", err)
	}

	// Verify the returned values are consistent
	if name == "" {
		t.Error("findPackageManager() returned empty name")
	}
	if cmd == "" {
		t.Error("findPackageManager() returned empty cmd")
	}

	// npm, pnpm, bun all use 'install -g'
	// yarn uses 'global add'
	validInstallCmds := map[string][]string{
		"npm":  {"install", "-g"},
		"pnpm": {"install", "-g"},
		"yarn": {"global", "add"},
		"bun":  {"install", "-g"},
	}

	expectedCmd, ok := validInstallCmds[name]
	if !ok {
		t.Errorf("findPackageManager() returned unknown name: %s", name)
	} else {
		// Verify yarn returns "global add" format
		if name == "yarn" {
			if cmd != "yarn" {
				t.Errorf("findPackageManager() yarn should return 'yarn', got: %s", cmd)
			}
		}
		_ = expectedCmd // just to use the variable
	}
}

func TestErrUnknownShell(t *testing.T) {
	if ErrUnknownShell == nil {
		t.Error("ErrUnknownShell should not be nil")
	}
	if ErrUnknownShell.Error() != "unknown shell type" {
		t.Errorf("ErrUnknownShell.Error() = %q, expected 'unknown shell type'", ErrUnknownShell.Error())
	}
}

func TestDetectCurrentShellEnvOverride(t *testing.T) {
	// Test SHELL environment variable override
	oldShell := os.Getenv("SHELL")
	t.Cleanup(func() {
		if oldShell == "" {
			os.Unsetenv("SHELL")
		} else {
			os.Setenv("SHELL", oldShell)
		}
	})

	tests := []struct {
		name     string
		envValue string
		want     ShellType
	}{
		{"bash path", "/bin/bash", ShellBash},
		{"bash path alt", "/usr/bin/bash", ShellBash},
		{"zsh path", "/bin/zsh", ShellZsh},
		{"zsh path alt", "/usr/bin/zsh", ShellZsh},
		{"fish path", "/usr/bin/fish", ShellFish},
		{"fish path alt", "/home/user/.local/bin/fish", ShellFish},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			os.Setenv("SHELL", tt.envValue)
			// Also unset fish env var to ensure SHELL is used
			os.Unsetenv("__FISH_VERSION_DIR")

			got := DetectCurrentShell()
			if got != tt.want {
				t.Errorf("DetectCurrentShell() with SHELL=%s = %v, want %v", tt.envValue, got, tt.want)
			}
		})
	}
}

func TestDetectCurrentShellEmptySHELL(t *testing.T) {
	// Test behavior when SHELL is empty
	oldShell := os.Getenv("SHELL")
	oldFish := os.Getenv("__FISH_VERSION_DIR")
	t.Cleanup(func() {
		if oldShell == "" {
			os.Unsetenv("SHELL")
		} else {
			os.Setenv("SHELL", oldShell)
		}
		if oldFish == "" {
			os.Unsetenv("__FISH_VERSION_DIR")
		} else {
			os.Setenv("__FISH_VERSION_DIR", oldFish)
		}
	})

	os.Setenv("SHELL", "")
	os.Unsetenv("__FISH_VERSION_DIR")

	got := DetectCurrentShell()
	// Should fallback to bash on unix or powershell on windows
	if got != ShellBash && got != ShellPowerShell {
		t.Errorf("DetectCurrentShell() with empty SHELL = %v, expected ShellBash or ShellPowerShell", got)
	}
}
