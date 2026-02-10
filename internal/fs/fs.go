// Package fs provides file system utilities for ply.
package fs

import (
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"strings"
)

var (
	// ErrMasterKeyNotFound is returned when the master key file doesn't exist.
	ErrMasterKeyNotFound = errors.New("master key not found")

	// ErrDataDirNotFound is returned when the data directory doesn't exist.
	ErrDataDirNotFound = errors.New("data directory not found")

	// ErrDefaultNotFound is returned when default provider is not set.
	ErrDefaultNotFound = errors.New("default provider not found")
)

const (
	dataDir     = ".local/share/ply"
	masterKey   = "master.key"
	defaultFile = "default.txt"
)

// DataDir returns the path to the ply data directory.
func DataDir() (string, error) {
	home, err := os.UserHomeDir()
	if err != nil {
		return "", fmt.Errorf("failed to get home directory: %w", err)
	}
	return filepath.Join(home, dataDir), nil
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

// DefaultProviderPath returns the path to the default provider file.
func DefaultProviderPath() (string, error) {
	dataDir, err := DataDir()
	if err != nil {
		return "", err
	}
	return filepath.Join(dataDir, defaultFile), nil
}

// LoadDefaultProvider loads the default provider ID from file.
func LoadDefaultProvider() (string, error) {
	path, err := DefaultProviderPath()
	if err != nil {
		return "", err
	}

	data, err := os.ReadFile(path)
	if err != nil {
		if os.IsNotExist(err) {
			return "", ErrDefaultNotFound
		}
		return "", fmt.Errorf("failed to read default provider: %w", err)
	}

	// Trim whitespace and newlines
	id := strings.TrimSpace(string(data))

	if id == "" {
		return "", ErrDefaultNotFound
	}

	return id, nil
}

// SaveDefaultProvider saves the default provider ID to file.
func SaveDefaultProvider(id string) error {
	path, err := DefaultProviderPath()
	if err != nil {
		return err
	}

	if err := os.WriteFile(path, []byte(id), 0600); err != nil {
		return fmt.Errorf("failed to write default provider: %w", err)
	}

	return nil
}

// ClearDefaultProvider removes the default provider file.
func ClearDefaultProvider() error {
	path, err := DefaultProviderPath()
	if err != nil {
		return err
	}

	if err := os.Remove(path); err != nil && !os.IsNotExist(err) {
		return fmt.Errorf("failed to clear default provider: %w", err)
	}

	return nil
}
