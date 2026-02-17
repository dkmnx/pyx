// Package fs provides file system utilities for ply.
package fs

import (
	"errors"
	"fmt"
	"os"
	"path/filepath"
)

var (
	// ErrMasterKeyNotFound is returned when the master key file doesn't exist.
	ErrMasterKeyNotFound = errors.New("master key not found")

	// ErrDataDirNotFound is returned when the data directory doesn't exist.
	ErrDataDirNotFound = errors.New("data directory not found")
)

const (
	appName   = "ply"
	masterKey = "master.key"
)

// DataDir returns the path to the ply data directory.
// Follows XDG Base Directory spec: uses $XDG_DATA_HOME/ply if set,
// otherwise defaults to ~/.local/share/ply.
func DataDir() (string, error) {
	xdgDataHome := os.Getenv("XDG_DATA_HOME")
	if xdgDataHome != "" {
		return filepath.Join(xdgDataHome, appName), nil
	}

	home, err := os.UserHomeDir()
	if err != nil {
		return "", fmt.Errorf("failed to get home directory: %w", err)
	}
	return filepath.Join(home, ".local", "share", appName), nil
}

// MasterKeyPath returns the path to the master key file.
func MasterKeyPath() (string, error) {
	dataDir, err := DataDir()
	if err != nil {
		return "", err
	}
	return filepath.Join(dataDir, masterKey), nil
}

// EnsureDataDir creates the data directory if it doesn't exist.
func EnsureDataDir() (string, error) {
	dataDir, err := DataDir()
	if err != nil {
		return "", err
	}

	if err := os.MkdirAll(dataDir, 0700); err != nil {
		return "", fmt.Errorf("failed to create data directory: %w", err)
	}

	return dataDir, nil
}

// LoadMasterKey loads the master key from file.
func LoadMasterKey() ([]byte, error) {
	path, err := MasterKeyPath()
	if err != nil {
		return nil, err
	}

	data, err := os.ReadFile(path)
	if err != nil {
		if os.IsNotExist(err) {
			return nil, ErrMasterKeyNotFound
		}
		return nil, fmt.Errorf("failed to read master key: %w", err)
	}

	return data, nil
}

// SaveMasterKey saves the master key to file with restricted permissions.
func SaveMasterKey(key []byte) error {
	path, err := MasterKeyPath()
	if err != nil {
		return err
	}

	if err := os.WriteFile(path, key, 0600); err != nil {
		return fmt.Errorf("failed to write master key: %w", err)
	}

	return nil
}

// MasterKeyExists checks if the master key file exists.
func MasterKeyExists() (bool, error) {
	path, err := MasterKeyPath()
	if err != nil {
		return false, err
	}

	_, err = os.Stat(path)
	if err != nil {
		if os.IsNotExist(err) {
			return false, nil
		}
		return false, err
	}

	return true, nil
}

// DataDirExists checks if the data directory exists.
func DataDirExists() (bool, error) {
	dataDir, err := DataDir()
	if err != nil {
		return false, err
	}

	_, err = os.Stat(dataDir)
	if err != nil {
		if os.IsNotExist(err) {
			return false, nil
		}
		return false, err
	}

	return true, nil
}
