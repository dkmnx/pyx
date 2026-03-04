// Package pi provides utilities for checking and installing pi coding agent.
package pi

import (
	"context"
	"errors"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strings"

	"github.com/dkmnx/ply/internal/prompt"
)

const (
	// NPMPackage is npm package name for pi
	NPMPackage = "@mariozechner/pi-coding-agent"
	// BinaryName is name of pi executable
	BinaryName = "pi"
	// packageManagerYarn is name for yarn package manager
	packageManagerYarn = "yarn"
)

// ShellType represents the detected shell type.
type ShellType string

const (
	// ShellBash represents the bash shell
	ShellBash ShellType = "bash"
	// ShellZsh represents the zsh shell
	ShellZsh ShellType = "zsh"
	// ShellFish represents the fish shell
	ShellFish ShellType = "fish"
	// ShellPowerShell represents PowerShell
	ShellPowerShell ShellType = "powershell"
)

// ShellNames contains all valid shell type names
var ShellNames = []string{string(ShellBash), string(ShellZsh), string(ShellFish), string(ShellPowerShell)}

// DetectCurrentShell detects the current shell environment.
// Returns the detected shell type.
func DetectCurrentShell() ShellType {
	// Check shell environment variable
	shell := os.Getenv("SHELL")
	if shell != "" {
		shell = filepath.Base(shell)
		switch shell {
		case "bash":
			return ShellBash
		case "zsh":
			return ShellZsh
		case "fish":
			return ShellFish
		}
	}

	// Fallback: check for fish-specific environment variables
	if os.Getenv("__FISH_VERSION_DIR") != "" {
		return ShellFish
	}

	// Fallback: assume bash on Unix-like systems
	if runtime.GOOS == "linux" || runtime.GOOS == "darwin" {
		return ShellBash
	}

	// Default to powershell on Windows
	if runtime.GOOS == "windows" {
		return ShellPowerShell
	}

	// Final fallback to bash
	return ShellBash
}

// getHomeDir returns the user's home directory or empty string on error
func getHomeDir() (string, error) {
	home, err := os.UserHomeDir()
	if err != nil {
		return "", fmt.Errorf("failed to get home directory: %w", err)
	}
	return home, nil
}

// ErrUnknownShell is returned when the shell type is not recognized
var ErrUnknownShell = errors.New("unknown shell type")

// CompletionScriptPath returns the completion script path for a shell.
// Returns an error if the home directory cannot be determined or shell is unknown.
func CompletionScriptPath(shell ShellType) (string, error) {
	home, err := getHomeDir()
	if err != nil {
		return "", err
	}

	switch shell {
	case ShellZsh:
		// Zsh: ${fpath[1]}/_ply
		return filepath.Join(home, ".zshrc"), nil
	case ShellFish:
		// Fish: ~/.config/fish/completions/ply.fish
		return filepath.Join(home, ".config", "fish", "completions", "ply.fish"), nil
	case ShellPowerShell:
		// PowerShell: ply.ps1 in user's Documents/PowerShell
		return filepath.Join(home, "Documents", "PowerShell", "ply.ps1"), nil
	case ShellBash:
		// Bash: try both system and user locations
		// System: /etc/bash_completion.d/ply (Linux)
		// User: ~/.bashrc
		return filepath.Join(home, ".bashrc"), nil
	default:
		return "", ErrUnknownShell
	}
}

// InstallCompletion installs the appropriate completion script for the detected shell.
func InstallCompletion() error {
	shell := DetectCurrentShell()

	// Generate completion script using ply
	cmd := exec.Command("ply", "completion", string(shell))
	output, err := cmd.CombinedOutput()
	if err != nil {
		return fmt.Errorf("failed to generate completion script: %w", err)
	}

	// Get the actual completion script install path
	scriptPath, err := completionScriptInstallPath(shell)
	if err != nil {
		if errors.Is(err, ErrUnknownShell) {
			fmt.Printf("Completion installation not available for %s shell\n", shell)
			return nil
		}
		return fmt.Errorf("failed to get install path: %w", err)
	}

	// Skip if already installed (check actual completion script, not config file)
	if _, err := os.Stat(scriptPath); err == nil {
		return nil
	}

	// Ensure directory exists
	dir := filepath.Dir(scriptPath)
	if err := os.MkdirAll(dir, 0700); err != nil {
		return fmt.Errorf("failed to create directory: %w", err)
	}

	// Write completion script (0600 for security-sensitive files)
	if err := os.WriteFile(scriptPath, output, 0600); err != nil {
		return fmt.Errorf("failed to write completion script: %w", err)
	}

	fmt.Printf("✓ Completion script installed for %s shell\n", shell)
	fmt.Printf("  Script location: %s\n", scriptPath)

	// Add source/activation instruction for zsh
	if shell == ShellZsh {
		fmt.Println("  To enable completions, restart your shell or run:")
		fmt.Println("    autoload -U compinit; compinit")
	} else if shell == ShellFish {
		fmt.Println("  To enable completions, restart your shell or run:")
		fmt.Println("    source \"" + scriptPath + "\"")
	} else if shell == ShellPowerShell {
		fmt.Println("  To enable completions for every new session, add to your profile:")
		fmt.Println("    Add-Content -Path $PROFILE -Value '. " + scriptPath + "'")
	} else {
		fmt.Println("  Completions will be loaded automatically")
	}

	return nil
}

// completionScriptInstallPath returns the path where completion scripts are installed.
func completionScriptInstallPath(shell ShellType) (string, error) {
	home, err := os.UserHomeDir()
	if err != nil {
		return "", fmt.Errorf("failed to get home directory: %w", err)
	}

	switch shell {
	case ShellBash:
		return filepath.Join(home, ".bash_completions", "ply.bash"), nil
	case ShellZsh:
		return filepath.Join(home, ".zsh", "completions", "_ply"), nil
	case ShellFish:
		return filepath.Join(home, ".config", "fish", "completions", "ply.fish"), nil
	case ShellPowerShell:
		return filepath.Join(home, "Documents", "PowerShell", "ply.ps1"), nil
	default:
		return "", ErrUnknownShell
	}
}

// CheckInstalled checks if pi is installed and available on PATH.
func CheckInstalled() (bool, error) {
	_, err := exec.LookPath(BinaryName)
	if err != nil {
		return false, nil
	}
	return true, nil
}

// Install installs pi using the specified package manager.
// If packageManager is empty, auto-detects the first available package manager.
func Install(packageManager string) error {
	// If no package manager specified, find one
	if packageManager == "" {
		var err error
		packageManager, _, err = findPackageManager()
		if err != nil {
			return fmt.Errorf("no compatible package manager found: %w", err)
		}
	}

	// Verify the selected package manager is available
	if _, err := exec.LookPath(packageManager); err != nil {
		return fmt.Errorf("package manager '%s' not found: %w", packageManager, err)
	}

	// Confirm installation
	fmt.Printf("Installing pi using %s...\n", packageManager)

	// Run install command
	// yarn uses 'global add', others use 'install -g'
	var installCmd *exec.Cmd
	if packageManager == packageManagerYarn {
		installCmd = exec.Command(packageManager, "global", "add", NPMPackage)
	} else {
		installCmd = exec.Command(packageManager, "install", "-g", NPMPackage)
	}
	installCmd.Stdout = os.Stdout
	installCmd.Stderr = os.Stderr
	if err := installCmd.Run(); err != nil {
		return fmt.Errorf("failed to install pi: %w", err)
	}

	fmt.Println("✓ pi installed successfully")
	return nil
}

// EnsureInstalled checks if pi is installed and installs it if not.
// Prompts for package manager selection if installation is needed.
// Returns true if pi was installed, false if it was already installed.
func EnsureInstalled(ctx context.Context) (bool, error) {
	installed, err := CheckInstalled()
	if err != nil {
		return false, fmt.Errorf("failed to check if pi is installed: %w", err)
	}

	if installed {
		// Pi is installed, try to install completion for detected shell
		if err := InstallCompletion(); err != nil {
			fmt.Printf("Warning: failed to install shell completions: %v\n", err)
		}
		return false, nil
	}

	// Auto-install with package manager prompt
	fmt.Println("pi is not installed.")

	// Prompt for package manager selection
	packageManager, err := prompt.PromptPackageManager(ctx)
	if err != nil {
		return false, fmt.Errorf("failed to select package manager: %w", err)
	}

	if err := Install(packageManager); err != nil {
		return false, fmt.Errorf("failed to install pi: %w", err)
	}

	// Pi installed successfully, install completion
	if err := InstallCompletion(); err != nil {
		fmt.Printf("Warning: failed to install shell completions: %v\n", err)
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
		return packageManagerYarn, packageManagerYarn, nil
	}

	// Check bun
	if _, err := exec.LookPath("bun"); err == nil {
		// bun uses 'install -g' like npm
		return "bun", "bun", nil
	}

	return "", "", fmt.Errorf("no package manager found (npm, pnpm, yarn, or bun required)")
}

// InstallCommand returns to command string to install pi.
// This is useful for showing to user what command to run.
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
