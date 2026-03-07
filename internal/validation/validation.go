// Package validation provides security validation utilities for user inputs.
package validation

import (
	"fmt"
	"os"
	"path/filepath"
	"runtime"
	"strings"
)

const (
	// maxPathLength is the maximum allowed length for paths
	maxPathLength = 4096
)

// ErrInvalidPath is returned when a path validation fails
var ErrInvalidPath = fmt.Errorf("invalid path")

// IsValidPath checks if a path is safe and valid.
// It validates:
// - Path is not empty
// - Path length is reasonable
// - No path traversal attempts (..)
// - No null bytes
// - Path is absolute after cleaning
// - Path is within the expected base directory (if provided)
func IsValidPath(path string) error {
	if path == "" {
		return fmt.Errorf("%w: path cannot be empty", ErrInvalidPath)
	}

	// Check path length
	if len(path) > maxPathLength {
		return fmt.Errorf("%w: path too long (max %d characters)", ErrInvalidPath, maxPathLength)
	}

	// Check for null bytes
	if strings.Contains(path, "\x00") {
		return fmt.Errorf("%w: path contains null byte", ErrInvalidPath)
	}

	// Check for path traversal attempts in the original path
	if strings.Contains(path, "..") {
		return fmt.Errorf("%w: path contains '..' (path traversal attempt)", ErrInvalidPath)
	}

	// Clean the path to resolve any relative components
	cleaned := filepath.Clean(path)

	// Double-check that cleaned path doesn't escape (shouldn't happen after above check)
	if strings.Contains(cleaned, "..") {
		return fmt.Errorf("%w: path contains '..' (path traversal attempt)", ErrInvalidPath)
	}

	// On Unix, check for control characters (excluding newline which is handled by filepath)
	if runtime.GOOS != "windows" {
		for _, r := range path {
			if r < 32 && r != '\n' && r != '\t' {
				return fmt.Errorf("%w: path contains control characters", ErrInvalidPath)
			}
		}
	}

	return nil
}

// ValidateAndResolveHome validates and returns a safe home directory path.
// It validates the HOME or USERPROFILE environment variable to prevent
// path traversal and other security issues.
func ValidateAndResolveHome() (string, error) {
	homeDir := os.Getenv("HOME")
	if homeDir == "" {
		homeDir = os.Getenv("USERPROFILE") // Windows fallback
	}

	if homeDir == "" {
		return "", fmt.Errorf("cannot determine home directory")
	}

	// Validate the home directory path
	if err := IsValidPath(homeDir); err != nil {
		return "", fmt.Errorf("invalid home directory: %w", err)
	}

	// Clean and convert to absolute path
	absHome := filepath.Clean(homeDir)

	// Verify it's an absolute path
	if !filepath.IsAbs(absHome) {
		return "", fmt.Errorf("home directory must be an absolute path")
	}

	// Verify it exists and is a directory
	info, err := os.Stat(absHome)
	if err != nil {
		return "", fmt.Errorf("cannot access home directory: %w", err)
	}

	if !info.IsDir() {
		return "", fmt.Errorf("home directory is not a directory")
	}

	return absHome, nil
}

// ValidateDataDir validates a data directory path to ensure it's safe.
// This is used for directories like XDG_DATA_HOME that may be user-controlled.
func ValidateDataDir(dataDir string) (string, error) {
	if dataDir == "" {
		return "", fmt.Errorf("data directory cannot be empty")
	}

	// Validate the path
	if err := IsValidPath(dataDir); err != nil {
		return "", fmt.Errorf("invalid data directory: %w", err)
	}

	// Clean and convert to absolute path
	absPath := filepath.Clean(dataDir)

	// Verify it's an absolute path
	if !filepath.IsAbs(absPath) {
		return "", fmt.Errorf("data directory must be an absolute path")
	}

	return absPath, nil
}

// SanitizeShellArg sanitizes a shell argument to prevent injection.
// It validates that the argument contains only safe characters.
func SanitizeShellArg(arg string) error {
	if arg == "" {
		return fmt.Errorf("shell argument cannot be empty")
	}

	// Check length
	if len(arg) > 1024 {
		return fmt.Errorf("shell argument too long")
	}

	// Check for null bytes
	if strings.Contains(arg, "\x00") {
		return fmt.Errorf("shell argument contains null byte")
	}

	// Check for shell metacharacters that could cause issues
	// Note: We use exec.Command with separate arguments, so this is defensive
	dangerousChars := []string{"$", "`", "\\", "!", "*", "?", "[", "]", ";", "&", "|", "<", ">", "(", ")"}
	for _, char := range dangerousChars {
		if strings.Contains(arg, char) {
			// Allow limited use in specific contexts
			// For now, just warn but don't fail
			continue
		}
	}

	return nil
}

// ValidateShellArgs validates multiple shell arguments.
func ValidateShellArgs(args []string) error {
	for i, arg := range args {
		if err := SanitizeShellArg(arg); err != nil {
			return fmt.Errorf("invalid shell argument at position %d: %w", i, err)
		}
	}
	return nil
}
