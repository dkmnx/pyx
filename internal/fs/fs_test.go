package fs

import (
	"os"
	"path/filepath"
	"runtime"
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

	// Verify permissions - skip on Windows as it doesn't support Unix permissions
	if runtime.GOOS != "windows" {
		if info.Mode().Perm() != 0700 {
			t.Errorf("EnsureDataDir() permissions = %v, want 0700", info.Mode().Perm())
		}
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

	// Check file permissions - skip on Windows as it doesn't support Unix permissions
	if runtime.GOOS != "windows" {
		masterKeyPath, _ := MasterKeyPath()
		info, err := os.Stat(masterKeyPath)
		if err != nil {
			t.Fatalf("os.Stat() error = %v", err)
		}

		if info.Mode().Perm() != 0600 {
			t.Errorf("SaveMasterKey() file permissions = %v, want 0600", info.Mode().Perm())
		}
	}
}

func TestLoadMasterKeyNotFound(t *testing.T) {
	// Use a temp directory for testing
	origHome := os.Getenv("HOME")
	defer os.Setenv("HOME", origHome)

	// Create a unique temp directory for this test
	tempHome := t.TempDir()
	os.Setenv("HOME", tempHome)

	// Use XDG_DATA_HOME which is respected on all platforms
	origXDG := os.Getenv("XDG_DATA_HOME")
	defer os.Setenv("XDG_DATA_HOME", origXDG)
	os.Setenv("XDG_DATA_HOME", tempHome)

	_, err := LoadMasterKey()
	if err != ErrMasterKeyNotFound {
		t.Errorf("LoadMasterKey() error = %v, want %v", err, ErrMasterKeyNotFound)
	}
}

func TestMasterKeyExists(t *testing.T) {
	// Use a temp directory for testing
	origHome := os.Getenv("HOME")
	defer os.Setenv("HOME", origHome)

	// Create a unique temp directory for this test
	tempHome := t.TempDir()
	os.Setenv("HOME", tempHome)

	// Use XDG_DATA_HOME which is respected on all platforms
	origXDG := os.Getenv("XDG_DATA_HOME")
	defer os.Setenv("XDG_DATA_HOME", origXDG)
	os.Setenv("XDG_DATA_HOME", tempHome)

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
	defer os.Setenv("HOME", origHome)

	// Create a unique temp directory for this test
	tempHome := t.TempDir()
	os.Setenv("HOME", tempHome)

	// Use XDG_DATA_HOME which is respected on all platforms
	origXDG := os.Getenv("XDG_DATA_HOME")
	defer os.Setenv("XDG_DATA_HOME", origXDG)
	os.Setenv("XDG_DATA_HOME", tempHome)

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
