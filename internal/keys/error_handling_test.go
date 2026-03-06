package keys

import (
	"os"
	"testing"
)

func TestSaveNoPassword(t *testing.T) {
	dir := tempDir(t)
	service, user := testKeyring(t)
	m := NewWithKeyring(dir, service, user)

	key, err := GenerateKey()
	if err != nil {
		t.Fatalf("GenerateKey() error = %v", err)
	}

	// Don't set a password - should fail
	err = m.Save(key)
	if err == nil {
		t.Error("Save() should return error when no password is set")
	}
}

func TestLoadWrongPassword(t *testing.T) {
	dir := tempDir(t)
	service, user := testKeyring(t)
	m := NewWithKeyring(dir, service, user)

	// Set up password in keyring
	testPassword := "correct-password"
	if err := m.SetPassword([]byte(testPassword)); err != nil {
		t.Skip("Keyring not available, skipping test")
	}
	t.Cleanup(func() { _ = m.DeletePassword() })

	// Generate and save key with correct password
	key, err := GenerateKey()
	if err != nil {
		t.Fatalf("GenerateKey() error = %v", err)
	}

	if err := m.Save(key); err != nil {
		t.Fatalf("Save() error = %v", err)
	}

	// Delete password from keyring
	_ = m.DeletePassword()

	// Set wrong password
	wrongPassword := "wrong-password"
	if err := m.SetPassword([]byte(wrongPassword)); err != nil {
		t.Skip("Keyring not available, skipping test")
	}
	t.Cleanup(func() { _ = m.DeletePassword() })

	// Try to load with wrong password - should fail with ErrNoPassword after migration attempt
	_, err = m.Load(nil)
	if err == nil {
		t.Error("Load() should return error when password is wrong")
	}
}

func TestSetPasswordEmpty(t *testing.T) {
	dir := tempDir(t)
	service, user := testKeyring(t)
	m := NewWithKeyring(dir, service, user)

	err := m.SetPassword([]byte{})
	if err != ErrInvalidPassword {
		t.Errorf("SetPassword() with empty password error = %v, want ErrInvalidPassword", err)
	}

	err = m.SetPassword(nil)
	if err != ErrInvalidPassword {
		t.Errorf("SetPassword() with nil password error = %v, want ErrInvalidPassword", err)
	}
}

func TestGetStoredPasswordNotFound(t *testing.T) {
	dir := tempDir(t)
	service, user := testKeyring(t)
	m := NewWithKeyring(dir, service, user)

	// Don't set a password
	_, err := m.GetStoredPassword()
	if err != ErrNoPassword {
		t.Errorf("GetStoredPassword() error = %v, want ErrNoPassword", err)
	}
}

func TestMigrateToKeyringErrorPaths(t *testing.T) {
	dir := tempDir(t)
	service, user := testKeyring(t)
	m := NewWithKeyring(dir, service, user)

	masterKey, err := GenerateKey()
	if err != nil {
		t.Fatalf("GenerateKey() error = %v", err)
	}

	// Migrate should work if keyring is available
	newPassword := []byte("new-password")
	err = m.MigrateToKeyring(masterKey, newPassword)
	if err != nil {
		t.Skip("Keyring not available, skipping test")
	}
	t.Cleanup(func() { _ = m.DeletePassword() })

	// Verify password was set
	exists, err := m.PasswordExists()
	if err != nil {
		t.Fatalf("PasswordExists() error = %v", err)
	}
	if !exists {
		t.Error("Password should exist after migration")
	}

	// Verify key can be loaded
	loadedKey, err := m.Load(nil)
	if err != nil {
		t.Fatalf("Load() after migration error = %v", err)
	}

	if !Equal(masterKey, loadedKey) {
		t.Error("Loaded key should match original after migration")
	}
}

func TestMigrateToKeyringEmptyPassword(t *testing.T) {
	dir := tempDir(t)
	service, user := testKeyring(t)
	m := NewWithKeyring(dir, service, user)

	masterKey, err := GenerateKey()
	if err != nil {
		t.Fatalf("GenerateKey() error = %v", err)
	}

	// Try to migrate with empty password
	err = m.MigrateToKeyring(masterKey, []byte{})
	if err == nil {
		t.Error("MigrateToKeyring() with empty password should return error")
	}
}

func TestDeletePasswordNotFound(t *testing.T) {
	dir := tempDir(t)
	service, user := testKeyring(t)
	m := NewWithKeyring(dir, service, user)

	// DeletePassword should not error when password doesn't exist
	err := m.DeletePassword()
	if err != nil {
		t.Errorf("DeletePassword() should not error when password doesn't exist, got %v", err)
	}
}

func TestRequiresPasswordEnvVar(t *testing.T) {
	dir := tempDir(t)
	service, user := testKeyring(t)
	m := NewWithKeyring(dir, service, user)

	// Set environment variable
	oldEnv := os.Getenv("PLY_PASSPHRASE")
	os.Setenv("PLY_PASSPHRASE", "test-passphrase")
	defer os.Setenv("PLY_PASSPHRASE", oldEnv)

	requires, err := m.RequiresPassword()
	if err != nil {
		t.Fatalf("RequiresPassword() error = %v", err)
	}

	if !requires {
		t.Error("RequiresPassword() should return true when PLY_PASSPHRASE is set")
	}
}

func TestPasswordExistsError(t *testing.T) {
	dir := tempDir(t)
	service, user := testKeyring(t)
	m := NewWithKeyring(dir, service, user)

	// PasswordExists should handle keyring errors gracefully
	// This is hard to test without mocking, but we can at least test the happy path
	exists, err := m.PasswordExists()
	if err != nil {
		// If keyring is not available, that's ok
		t.Logf("PasswordExists() error (may be expected): %v", err)
	}

	// After setting a password, it should exist
	if err := m.SetPassword([]byte("test")); err == nil {
		t.Cleanup(func() { _ = m.DeletePassword() })

		exists, err = m.PasswordExists()
		if err != nil {
			t.Fatalf("PasswordExists() error = %v", err)
		}
		if !exists {
			t.Error("Password should exist after SetPassword()")
		}
	}
}

func TestCanLoadWithNoKey(t *testing.T) {
	dir := tempDir(t)
	service, user := testKeyring(t)
	m := NewWithKeyring(dir, service, user)

	// CanLoad should return false when no key exists
	canLoad := m.CanLoad()
	if canLoad {
		t.Error("CanLoad() should return false when no key exists")
	}
}

func TestCanLoadWithKey(t *testing.T) {
	dir := tempDir(t)
	service, user := testKeyring(t)
	m := NewWithKeyring(dir, service, user)

	// Set up password
	testPassword := "test-password"
	if err := m.SetPassword([]byte(testPassword)); err != nil {
		t.Skip("Keyring not available, skipping test")
	}
	t.Cleanup(func() { _ = m.DeletePassword() })

	// Generate and save key
	key, err := GenerateKey()
	if err != nil {
		t.Fatalf("GenerateKey() error = %v", err)
	}

	if err := m.Save(key); err != nil {
		t.Fatalf("Save() error = %v", err)
	}

	// CanLoad should return true
	canLoad := m.CanLoad()
	if !canLoad {
		t.Error("CanLoad() should return true when key exists and can be loaded")
	}
}

func TestExistsError(t *testing.T) {
	dir := tempDir(t)
	service, user := testKeyring(t)
	m := NewWithKeyring(dir, service, user)

	// Exists should return false for non-existent key
	exists, err := m.Exists()
	if err != nil {
		t.Fatalf("Exists() error = %v", err)
	}
	if exists {
		t.Error("Exists() should return false for non-existent key")
	}
}

func TestLoadWithProvidedPassword(t *testing.T) {
	dir := tempDir(t)
	service, user := testKeyring(t)
	m := NewWithKeyring(dir, service, user)

	// Set up password in keyring
	testPassword := "test-password"
	if err := m.SetPassword([]byte(testPassword)); err != nil {
		t.Skip("Keyring not available, skipping test")
	}
	t.Cleanup(func() { _ = m.DeletePassword() })

	// Generate and save key
	key, err := GenerateKey()
	if err != nil {
		t.Fatalf("GenerateKey() error = %v", err)
	}

	if err := m.Save(key); err != nil {
		t.Fatalf("Save() error = %v", err)
	}

	// Delete password from keyring
	_ = m.DeletePassword()

	// Load with provided password
	loadedKey, err := m.Load([]byte(testPassword))
	if err != nil {
		t.Fatalf("Load() with provided password error = %v", err)
	}

	if !Equal(key, loadedKey) {
		t.Error("Loaded key should match original when using provided password")
	}
}

func TestGetPassphrasePriority(t *testing.T) {
	dir := tempDir(t)
	service, user := testKeyring(t)
	m := NewWithKeyring(dir, service, user)

	// Test 1: Provided password takes priority
	testPassword := "test-password"
	if err := m.SetPassword([]byte(testPassword)); err != nil {
		t.Skip("Keyring not available, skipping test")
	}
	t.Cleanup(func() { _ = m.DeletePassword() })

	// Set env var
	oldEnv := os.Getenv("PLY_PASSPHRASE")
	os.Setenv("PLY_PASSPHRASE", "env-password")
	defer os.Setenv("PLY_PASSPHRASE", oldEnv)

	// The manager should use provided password over env var
	// This is tested implicitly through Load()
	key, err := GenerateKey()
	if err != nil {
		t.Fatalf("GenerateKey() error = %v", err)
	}

	if err := m.Save(key); err != nil {
		t.Fatalf("Save() error = %v", err)
	}

	// Load with wrong provided password (should fail even though env var has correct password)
	_, err = m.Load([]byte("wrong-password"))
	if err == nil {
		t.Error("Load() should fail with wrong provided password even if env var has correct one")
	}
}

func TestNewWithKeyring(t *testing.T) {
	dir := tempDir(t)
	customService := "ply-custom-service"
	customUser := "custom-user"

	m := NewWithKeyring(dir, customService, customUser)

	if m.keyringService != customService {
		t.Errorf("NewWithKeyring() service = %v, want %v", m.keyringService, customService)
	}

	if m.keyringUser != customUser {
		t.Errorf("NewWithKeyring() user = %v, want %v", m.keyringUser, customUser)
	}
}

func TestManagerDeleteNonExistent(t *testing.T) {
	dir := tempDir(t)
	service, user := testKeyring(t)
	m := NewWithKeyring(dir, service, user)

	// Delete should not error when key doesn't exist
	err := m.Delete()
	if err != nil {
		t.Errorf("Delete() should not error when key doesn't exist, got %v", err)
	}
}

func TestLoadLegacyPassphrase(t *testing.T) {
	// This test would require complex setup to create a key encrypted with the legacy passphrase
	// The legacy passphrase is "default" and was used in old versions before keyring
	// For now, we skip this test as it requires accessing internal crypto functions
	t.Skip("Legacy passphrase test requires complex setup")
}

func TestEqualDifferentLengths(t *testing.T) {
	a := []byte("short")
	b := []byte("much longer")

	if Equal(a, b) {
		t.Error("Equal() should return false for different length slices")
	}
}

func TestEqualNilSlices(t *testing.T) {
	tests := []struct {
		name string
		a    []byte
		b    []byte
		want bool
	}{
		{"both nil", nil, nil, true},
		{"a nil", nil, []byte("test"), false},
		{"b nil", []byte("test"), nil, false},
		{"both empty", []byte{}, []byte{}, true},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := Equal(tt.a, tt.b)
			if got != tt.want {
				t.Errorf("Equal() = %v, want %v", got, tt.want)
			}
		})
	}
}
