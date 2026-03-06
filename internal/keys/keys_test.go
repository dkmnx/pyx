package keys

import (
	"os"
	"path/filepath"
	"testing"

	"github.com/google/uuid"
)

// testKeyring returns unique keyring service and user names for test isolation.
// This prevents tests from interfering with each other and with the user's actual installation.
func testKeyring(t *testing.T) (service, user string) {
	t.Helper()
	id := uuid.New().String()
	return "ply-test-" + id[:8], "test-key"
}

func tempDir(t *testing.T) string {
	t.Helper()
	dir := filepath.Join(os.TempDir(), "ply-test-"+uuid.New().String())
	if err := os.MkdirAll(dir, 0700); err != nil {
		t.Fatalf("failed to create temp dir: %v", err)
	}
	t.Cleanup(func() {
		os.RemoveAll(dir)
	})
	return dir
}

func TestGenerateKey(t *testing.T) {
	key, err := GenerateKey()
	if err != nil {
		t.Fatalf("GenerateKey() error = %v", err)
	}

	if len(key) != keySize {
		t.Errorf("GenerateKey() key length = %d, want %d", len(key), keySize)
	}

	// Generate another key to ensure they're different
	key2, err := GenerateKey()
	if err != nil {
		t.Fatalf("GenerateKey() error = %v", err)
	}

	if Equal(key, key2) {
		t.Error("GenerateKey() generated identical keys (statistically impossible)")
	}
}

func TestManagerSaveLoad(t *testing.T) {
	dir := tempDir(t)
	service, user := testKeyring(t)
	m := NewWithKeyring(dir, service, user)

	// Set up password in keyring for testing
	testPassword := "test-password-123"
	if err := m.SetPassword([]byte(testPassword)); err != nil {
		t.Skip("Keyring not available, skipping test")
	}
	// Clean up keyring after test
	t.Cleanup(func() { _ = m.DeletePassword() })

	key, err := GenerateKey()
	if err != nil {
		t.Fatalf("GenerateKey() error = %v", err)
	}

	// Save key
	if err := m.Save(key); err != nil {
		t.Fatalf("Save() error = %v", err)
	}

	// Check that key exists
	exists, err := m.Exists()
	if err != nil {
		t.Fatalf("Exists() error = %v", err)
	}
	if !exists {
		t.Error("Exists() returned false after saving key")
	}

	// Load key (password will be retrieved from keyring)
	loadedKey, err := m.Load(nil)
	if err != nil {
		t.Fatalf("Load() error = %v", err)
	}

	if !Equal(key, loadedKey) {
		t.Error("Load() returned different key than saved")
	}

	// Delete key
	if err := m.Delete(); err != nil {
		t.Fatalf("Delete() error = %v", err)
	}

	// Verify key doesn't exist
	exists, err = m.Exists()
	if err != nil {
		t.Fatalf("Exists() error = %v", err)
	}
	if exists {
		t.Error("Key should not exist after deletion")
	}
}

func TestManagerLoadNotFound(t *testing.T) {
	dir := tempDir(t)
	service, user := testKeyring(t)
	m := NewWithKeyring(dir, service, user)

	// Without a password set and no key file, should return ErrKeyNotFound
	_, err := m.Load(nil)
	if err == nil {
		t.Error("Load() should return error when key doesn't exist")
	}
	// Should return ErrKeyNotFound because there's no key file and no password
	if err != ErrKeyNotFound {
		t.Errorf("Load() error = %v, want ErrKeyNotFound", err)
	}
}

func TestManagerDelete(t *testing.T) {
	dir := tempDir(t)
	service, user := testKeyring(t)
	m := NewWithKeyring(dir, service, user)

	// Set up password in keyring for testing
	testPassword := "test-password-123"
	if err := m.SetPassword([]byte(testPassword)); err != nil {
		t.Skip("Keyring not available, skipping test")
	}
	// Clean up keyring after test
	t.Cleanup(func() { _ = m.DeletePassword() })

	key, err := GenerateKey()
	if err != nil {
		t.Fatalf("GenerateKey() error = %v", err)
	}

	// Save key
	if err := m.Save(key); err != nil {
		t.Fatalf("Save() error = %v", err)
	}

	// Delete key
	if err := m.Delete(); err != nil {
		t.Fatalf("Delete() error = %v", err)
	}

	// Verify key doesn't exist
	exists, err := m.Exists()
	if err != nil {
		t.Fatalf("Exists() error = %v", err)
	}
	if exists {
		t.Error("Key should not exist after deletion")
	}
}

func TestManagerRequiresPassword(t *testing.T) {
	dir := tempDir(t)
	service, user := testKeyring(t)
	m := NewWithKeyring(dir, service, user)

	// Without password, RequiresPassword should return false
	requires, err := m.RequiresPassword()
	if err != nil {
		t.Fatalf("RequiresPassword() error = %v", err)
	}
	if requires {
		t.Error("RequiresPassword() should return false when no password is set")
	}

	// Try to set a password in keyring
	testPassword := "test-password-123"
	if err := m.SetPassword([]byte(testPassword)); err != nil {
		t.Skip("Keyring not available, skipping test")
	}
	// Clean up keyring after test
	t.Cleanup(func() { _ = m.DeletePassword() })

	// With password set, RequiresPassword should return true
	requires, err = m.RequiresPassword()
	if err != nil {
		t.Fatalf("RequiresPassword() error = %v", err)
	}
	if !requires {
		t.Error("RequiresPassword() should return true when password is set")
	}
}

func TestEqual(t *testing.T) {
	tests := []struct {
		name string
		a    []byte
		b    []byte
		want bool
	}{
		{"equal slices", []byte("test"), []byte("test"), true},
		{"different slices", []byte("test"), []byte("different"), false},
		{"different lengths", []byte("test"), []byte("testing"), false},
		{"empty slices", []byte{}, []byte{}, true},
		{"nil slices", nil, nil, true},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			if got := Equal(tt.a, tt.b); got != tt.want {
				t.Errorf("Equal() = %v, want %v", got, tt.want)
			}
		})
	}
}
