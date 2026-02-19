// Package pi provides utilities for checking and installing the pi coding agent.
package pi

import (
	"fmt"
	"os"
	"os/exec"
	"runtime"
	"strings"
)

const (
	// NPMPackage is the npm package name for pi
	NPMPackage = "@mariozechner/pi-coding-agent"
	// BinaryName is the name of the pi executable
	BinaryName = "pi"
	// packageManagerYarn is the name for yarn package manager
	packageManagerYarn = "yarn"
)

// CheckInstalled checks if pi is installed and available on PATH.
func CheckInstalled() (bool, error) {
	_, err := exec.LookPath(BinaryName)
	if err != nil {
		return false, nil
	}
	return true, nil
}

// Install installs pi using npm.
// It checks which npm command is available (npm, pnpm, yarn, bun)
// and uses the appropriate command.
func Install() error {
	// Check which package manager is available
	pm, cmd, err := findPackageManager()
	if err != nil {
		return fmt.Errorf("no compatible package manager found: %w", err)
	}

	// Confirm installation
	fmt.Printf("Installing pi using %s...\n", pm)

	// Run install command
	installCmd := exec.Command(cmd, "install", "-g", NPMPackage)
	installCmd.Stdout = os.Stdout
	installCmd.Stderr = os.Stderr
	if err := installCmd.Run(); err != nil {
		return fmt.Errorf("failed to install pi: %w", err)
	}

	fmt.Println("✓ pi installed successfully")
	return nil
}

// EnsureInstalled checks if pi is installed, and installs it if not.
// Returns true if pi was installed, false if it was already installed.
func EnsureInstalled() (bool, error) {
	installed, err := CheckInstalled()
	if err != nil {
		return false, fmt.Errorf("failed to check if pi is installed: %w", err)
	}

	if installed {
		return false, nil
	}

	// Auto-install
	fmt.Println("pi is not installed.")
	fmt.Println("Installing...")

	if err := Install(); err != nil {
		return false, fmt.Errorf("failed to install pi: %w", err)
	}

	return true, nil
}

// findPackageManager determines which npm-compatible package manager is available.
// Returns (name, command, error)
func findPackageManager() (string, string, error) {
	// Check npm first
	if _, err := exec.LookPath("npm"); err == nil {
		return "npm", "npm", nil
	}

	// Check pnpm
	if _, err := exec.LookPath("pnpm"); err == nil {
		return "pnpm", "pnpm", nil
	}

	// Check yarn
	if _, err := exec.LookPath("yarn"); err == nil {
		// yarn uses 'global add' instead of 'install -g'
		return packageManagerYarn, "yarn", nil
	}

	// Check bun
	if _, err := exec.LookPath("bun"); err == nil {
		// bun uses 'install -g' like npm
		return "bun", "bun", nil
	}

	return "", "", fmt.Errorf("no package manager found (npm, pnpm, yarn, or bun required)")
}

// InstallCommand returns the command string to install pi.
// This is useful for showing the user what command to run.
func InstallCommand() string {
	pm, cmd, err := findPackageManager()
	if err != nil {
		// Default to npm as fallback
		return "npm install -g " + NPMPackage
	}

	switch pm {
	case packageManagerYarn:
		return fmt.Sprintf("%s global add %s", cmd, NPMPackage)
	default:
		// npm, pnpm, bun all use 'install -g'
		return fmt.Sprintf("%s install -g %s", cmd, NPMPackage)
	}
}

// Version returns the installed pi version.
func Version() (string, error) {
	cmd := exec.Command(BinaryName, "--version")
	output, err := cmd.CombinedOutput()
	if err != nil {
		return "", fmt.Errorf("failed to get pi version: %w", err)
	}

	version := strings.TrimSpace(string(output))
	return version, nil
}

// PlatformInfo returns information about the current platform.
// This is useful for debugging installation issues.
func PlatformInfo() string {
	return fmt.Sprintf("%s/%s", runtime.GOOS, runtime.GOARCH)
}
