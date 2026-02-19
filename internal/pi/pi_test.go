package pi

import (
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
