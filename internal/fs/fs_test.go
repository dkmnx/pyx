package fs

import (
	"os"
	"path/filepath"
	"testing"
)

func TestDataDir(t *testing.T) {
	dataDir, err := DataDir()
	if err != nil {
		t.Fatalf("DataDir() error = %v", err)
	}

	home, _ := os.UserHomeDir()
	expected := filepath.Join(home, ".local/share/ply")

	if dataDir != expected {
		t.Errorf("DataDir() = %v, want %v", dataDir, expected)
	}
}

func TestMasterKeyPath(t *testing.T) {
	masterKeyPath, err := MasterKeyPath()
	if err != nil {
		t.Fatalf("MasterKeyPath() error = %v", err)
	}

	home, _ := os.UserHomeDir()
	expected := filepath.Join(home, ".local/share/ply/master.key")

	if masterKeyPath != expected {
		t.Errorf("MasterKeyPath() = %v, want %v", masterKeyPath, expected)
	}
}

func TestEnsureDataDir(t *testing.T) {
	// Use a temp directory for testing
	origHome := os.Getenv("HOME")
	tempHome := t.TempDir()
	os.Setenv("HOME", tempHome)
	defer os.Setenv("HOME", origHome)

	dataDir, err := EnsureDataDir()
	if err != nil {
		t.Fatalf("EnsureDataDir() error = %v", err)
	}

	// Verify directory exists
	info, err := os.Stat(dataDir)
	if err != nil {
		t.Fatalf("os.Stat() error = %v", err)
	}

	if !info.IsDir() {
		t.Error("EnsureDataDir() did not create a directory")
	}

	// Verify permissions
	if info.Mode().Perm() != 0700 {
		t.Errorf("EnsureDataDir() permissions = %v, want 0700", info.Mode().Perm())
	}
}

func TestSaveAndLoadMasterKey(t *testing.T) {
	// Use a temp directory for testing
	origHome := os.Getenv("HOME")
	tempHome := t.TempDir()
	os.Setenv("HOME", tempHome)
	defer os.Setenv("HOME", origHome)

	// Ensure data directory exists first
	if _, err := EnsureDataDir(); err != nil {
		t.Fatalf("EnsureDataDir() error = %v", err)
	}

	key := []byte("test-master-key-32-bytes-long!")

	if err := SaveMasterKey(key); err != nil {
		t.Fatalf("SaveMasterKey() error = %v", err)
	}

	loaded, err := LoadMasterKey()
	if err != nil {
		t.Fatalf("LoadMasterKey() error = %v", err)
	}

	if string(loaded) != string(key) {
		t.Errorf("LoadMasterKey() = %v, want %v", loaded, key)
	}

	// Check file permissions
	masterKeyPath, _ := MasterKeyPath()
	info, err := os.Stat(masterKeyPath)
	if err != nil {
		t.Fatalf("os.Stat() error = %v", err)
	}

	if info.Mode().Perm() != 0600 {
		t.Errorf("SaveMasterKey() file permissions = %v, want 0600", info.Mode().Perm())
	}
}

func TestLoadMasterKeyNotFound(t *testing.T) {
	// Use a temp directory for testing
	origHome := os.Getenv("HOME")
	tempHome := t.TempDir()
	os.Setenv("HOME", tempHome)
	defer os.Setenv("HOME", origHome)

	_, err := LoadMasterKey()
	if err != ErrMasterKeyNotFound {
		t.Errorf("LoadMasterKey() error = %v, want %v", err, ErrMasterKeyNotFound)
	}
}

func TestMasterKeyExists(t *testing.T) {
	// Use a temp directory for testing
	origHome := os.Getenv("HOME")
	tempHome := t.TempDir()
	os.Setenv("HOME", tempHome)
	defer os.Setenv("HOME", origHome)

	exists, err := MasterKeyExists()
	if err != nil {
		t.Fatalf("MasterKeyExists() error = %v", err)
	}

	if exists {
		t.Error("MasterKeyExists() = true, want false (before creation)")
	}

	// Ensure data directory exists first
	if _, err := EnsureDataDir(); err != nil {
		t.Fatalf("EnsureDataDir() error = %v", err)
	}

	key := []byte("test-master-key-32-bytes-long!")
	if err := SaveMasterKey(key); err != nil {
		t.Fatalf("SaveMasterKey() error = %v", err)
	}

	exists, err = MasterKeyExists()
	if err != nil {
		t.Fatalf("MasterKeyExists() error = %v", err)
	}

	if !exists {
		t.Error("MasterKeyExists() = false, want true (after creation)")
	}
}

func TestDataDirExists(t *testing.T) {
	// Use a temp directory for testing
	origHome := os.Getenv("HOME")
	tempHome := t.TempDir()
	os.Setenv("HOME", tempHome)
	defer os.Setenv("HOME", origHome)

	exists, err := DataDirExists()
	if err != nil {
		t.Fatalf("DataDirExists() error = %v", err)
	}

	if exists {
		t.Error("DataDirExists() = true, want false (before creation)")
	}

	if _, err := EnsureDataDir(); err != nil {
		t.Fatalf("EnsureDataDir() error = %v", err)
	}

	exists, err = DataDirExists()
	if err != nil {
		t.Fatalf("DataDirExists() error = %v", err)
	}

	if !exists {
		t.Error("DataDirExists() = false, want true (after creation)")
	}
}

func TestDefaultProviderPath(t *testing.T) {
	// Use a temp directory for testing
	origHome := os.Getenv("HOME")
	tempHome := t.TempDir()
	os.Setenv("HOME", tempHome)
	defer os.Setenv("HOME", origHome)

	defaultPath, err := DefaultProviderPath()
	if err != nil {
		t.Fatalf("DefaultProviderPath() error = %v", err)
	}

	home, _ := os.UserHomeDir()
	expected := filepath.Join(home, ".local/share/ply/default.txt")

	if defaultPath != expected {
		t.Errorf("DefaultProviderPath() = %v, want %v", defaultPath, expected)
	}
}

func TestSaveAndLoadDefaultProvider(t *testing.T) {
	// Use a temp directory for testing
	origHome := os.Getenv("HOME")
	tempHome := t.TempDir()
	os.Setenv("HOME", tempHome)
	defer os.Setenv("HOME", origHome)

	// Ensure data directory exists first
	if _, err := EnsureDataDir(); err != nil {
		t.Fatalf("EnsureDataDir() error = %v", err)
	}

	providerID := "test-provider-id-12345"

	if err := SaveDefaultProvider(providerID); err != nil {
		t.Fatalf("SaveDefaultProvider() error = %v", err)
	}

	loaded, err := LoadDefaultProvider()
	if err != nil {
		t.Fatalf("LoadDefaultProvider() error = %v", err)
	}

	if loaded != providerID {
		t.Errorf("LoadDefaultProvider() = %v, want %v", loaded, providerID)
	}

	// Check file permissions
	defaultPath, _ := DefaultProviderPath()
	info, err := os.Stat(defaultPath)
	if err != nil {
		t.Fatalf("os.Stat() error = %v", err)
	}

	if info.Mode().Perm() != 0600 {
		t.Errorf("SaveDefaultProvider() file permissions = %v, want 0600", info.Mode().Perm())
	}
}

func TestLoadDefaultProviderNotFound(t *testing.T) {
	// Use a temp directory for testing
	origHome := os.Getenv("HOME")
	tempHome := t.TempDir()
	os.Setenv("HOME", tempHome)
	defer os.Setenv("HOME", origHome)

	_, err := LoadDefaultProvider()
	if err != ErrDefaultNotFound {
		t.Errorf("LoadDefaultProvider() error = %v, want %v", err, ErrDefaultNotFound)
	}
}

func TestLoadDefaultProviderWithWhitespace(t *testing.T) {
	// Use a temp directory for testing
	origHome := os.Getenv("HOME")
	tempHome := t.TempDir()
	os.Setenv("HOME", tempHome)
	defer os.Setenv("HOME", origHome)

	// Ensure data directory exists first
	if _, err := EnsureDataDir(); err != nil {
		t.Fatalf("EnsureDataDir() error = %v", err)
	}

	providerID := "test-provider-id-12345"
	providerIDWithNewline := providerID + "\n"

	if err := SaveDefaultProvider(providerIDWithNewline); err != nil {
		t.Fatalf("SaveDefaultProvider() error = %v", err)
	}

	loaded, err := LoadDefaultProvider()
	if err != nil {
		t.Fatalf("LoadDefaultProvider() error = %v", err)
	}

	if loaded != providerID {
		t.Errorf("LoadDefaultProvider() = %v, want %v (should trim newline)", loaded, providerID)
	}
}

func TestClearDefaultProvider(t *testing.T) {
	// Use a temp directory for testing
	origHome := os.Getenv("HOME")
	tempHome := t.TempDir()
	os.Setenv("HOME", tempHome)
	defer os.Setenv("HOME", origHome)

	// Ensure data directory exists first
	if _, err := EnsureDataDir(); err != nil {
		t.Fatalf("EnsureDataDir() error = %v", err)
	}

	providerID := "test-provider-id-12345"
	_ = SaveDefaultProvider(providerID)

	// Verify it exists
	_, err := LoadDefaultProvider()
	if err != nil {
		t.Fatalf("LoadDefaultProvider() should return ID before clear: %v", err)
	}

	// Clear the default
	if err := ClearDefaultProvider(); err != nil {
		t.Fatalf("ClearDefaultProvider() error = %v", err)
	}

	// Verify it's gone
	_, err = LoadDefaultProvider()
	if err != ErrDefaultNotFound {
		t.Errorf("LoadDefaultProvider() error = %v, want %v after clear", err, ErrDefaultNotFound)
	}
}

func TestClearDefaultProviderNotFound(t *testing.T) {
	// Use a temp directory for testing
	origHome := os.Getenv("HOME")
	tempHome := t.TempDir()
	os.Setenv("HOME", tempHome)
	defer os.Setenv("HOME", origHome)

	// Clearing when not set should not error
	if err := ClearDefaultProvider(); err != nil {
		t.Fatalf("ClearDefaultProvider() error when not set: %v", err)
	}
}
