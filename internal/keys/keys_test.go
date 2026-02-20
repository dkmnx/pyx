package keys

import (
	"errors"
	"os"
	"path/filepath"
	"testing"

	"github.com/dkmnx/ply/internal/database"
	"github.com/google/uuid"
	"github.com/zalando/go-keyring"
)

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

func TestManagerSaveLoadKeyring(t *testing.T) {
	// This test requires a functioning keyring
	// If keyring is unavailable, this test is skipped
	dir := tempDir(t)
	m := New(dir)

	key, err := GenerateKey()
	if err != nil {
		t.Fatalf("GenerateKey() error = %v", err)
	}

	// Try to save to keyring (will use file if keyring unavailable)
	if err := m.Save(key); err != nil {
		if errors.Is(err, ErrPasswordRequired) {
			t.Skip("keyring unavailable and no password set, skipping keyring test")
		}
		t.Fatalf("Save() error = %v", err)
	}

	// Try to load from keyring
	loadedKey, err := m.Load(nil)
	if err != nil {
		if errors.Is(err, ErrPasswordRequired) {
			t.Skip("keyring unavailable and no password set, skipping keyring test")
		}
		t.Fatalf("Load() error = %v", err)
	}

	if !Equal(key, loadedKey) {
		t.Error("Load() returned different key than saved")
	}

	// Cleanup
	_ = m.Delete()
}

func TestManagerSaveLoadFile(t *testing.T) {
	dir := tempDir(t)
	m := New(dir)

	key, err := GenerateKey()
	if err != nil {
		t.Fatalf("GenerateKey() error = %v", err)
	}

	password := []byte("test-password-12345")

	// Set password
	if err := m.SetPassword(password); err != nil {
		t.Fatalf("SetPassword() error = %v", err)
	}

	// Save key (will use file storage)
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

	// Check that password is required
	requires, err := m.RequiresPassword()
	if err != nil {
		t.Fatalf("RequiresPassword() error = %v", err)
	}
	if !requires {
		t.Error("RequiresPassword() returned false for file-based storage")
	}

	// Load with correct password
	loadedKey, err := m.Load(password)
	if err != nil {
		t.Fatalf("Load() with correct password error = %v", err)
	}

	if !Equal(key, loadedKey) {
		t.Error("Load() returned different key than saved")
	}

	// Try to load with wrong password
	wrongPassword := []byte("wrong-password")
	_, err = m.Load(wrongPassword)
	if err == nil {
		t.Error("Load() with wrong password should return error")
	}
	if !errors.Is(err, ErrInvalidPassword) {
		t.Errorf("Load() error = %v, want %v", err, ErrInvalidPassword)
	}

	// Try to load without password
	_, err = m.Load(nil)
	if err == nil {
		t.Error("Load() without password should return error")
	}
	if !errors.Is(err, ErrPasswordRequired) {
		t.Errorf("Load() error = %v, want %v", err, ErrPasswordRequired)
	}
}

func TestManagerSaveLoadFileNewPassword(t *testing.T) {
	dir := tempDir(t)
	m := New(dir)

	key, err := GenerateKey()
	if err != nil {
		t.Fatalf("GenerateKey() error = %v", err)
	}

	// Try to save without setting password first
	// This will succeed if keyring is available, so we need to check
	err = m.Save(key)
	if err == nil {
		// Keyring is available, which is fine for this test
		// Just verify the key can be loaded
		loadedKey, loadErr := m.Load(nil)
		if loadErr != nil {
			t.Fatalf("Load() error = %v", loadErr)
		}
		if !Equal(key, loadedKey) {
			t.Error("Load() returned different key than saved")
		}
		// Cleanup
		_ = m.Delete()
		return
	}

	// If error occurred, it should be ErrPasswordRequired
	if !errors.Is(err, ErrPasswordRequired) {
		t.Errorf("Save() error = %v, want ErrPasswordRequired", err)
	}
}

func TestManagerPasswordExists(t *testing.T) {
	dir := tempDir(t)
	m := New(dir)

	// Initially, no password
	exists, err := m.PasswordExists()
	if err != nil {
		t.Fatalf("PasswordExists() error = %v", err)
	}
	if exists {
		t.Error("PasswordExists() returned true before setting password")
	}

	// Set password
	password := []byte("test-password")
	if err := m.SetPassword(password); err != nil {
		t.Fatalf("SetPassword() error = %v", err)
	}

	// Now password should exist
	exists, err = m.PasswordExists()
	if err != nil {
		t.Fatalf("PasswordExists() error = %v", err)
	}
	if !exists {
		t.Error("PasswordExists() returned false after setting password")
	}
}

func TestManagerDelete(t *testing.T) {
	dir := tempDir(t)
	m := New(dir)

	key, err := GenerateKey()
	if err != nil {
		t.Fatalf("GenerateKey() error = %v", err)
	}

	password := []byte("test-password")

	// Set password and save key
	if err := m.SetPassword(password); err != nil {
		t.Fatalf("SetPassword() error = %v", err)
	}
	if err := m.Save(key); err != nil {
		t.Fatalf("Save() error = %v", err)
	}

	// Verify key exists
	exists, err := m.Exists()
	if err != nil {
		t.Fatalf("Exists() error = %v", err)
	}
	if !exists {
		t.Fatal("Key should exist before deletion")
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
	m := New(dir)

	_, err := m.Load(nil)
	if err == nil {
		t.Error("Load() should return error when key doesn't exist")
	}
	// Could be ErrKeyNotFound or ErrPasswordRequired
	if !errors.Is(err, ErrKeyNotFound) && !errors.Is(err, ErrPasswordRequired) {
		t.Errorf("Load() error = %v, want ErrKeyNotFound or ErrPasswordRequired", err)
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

func TestManagerMigrateFromLegacy(t *testing.T) {
	tests := []struct {
		name           string
		setupLegacy    bool
		legacyKeySize  int
		wantMigrated   bool
		shouldFailSave bool
	}{
		{
			name:           "successful migration",
			setupLegacy:    true,
			legacyKeySize:  keySize,
			wantMigrated:   true,
			shouldFailSave: false,
		},
		{
			name:           "no legacy key to migrate",
			setupLegacy:    false,
			legacyKeySize:  keySize,
			wantMigrated:   false,
			shouldFailSave: false,
		},
		{
			name:           "invalid legacy key size",
			setupLegacy:    true,
			legacyKeySize:  16, // Wrong size
			wantMigrated:   false,
			shouldFailSave: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			dir := tempDir(t)
			m := New(dir)

			// Cleanup keyring before and after test to ensure isolation
			t.Cleanup(func() {
				_ = keyring.Delete(keyringService, keyringUser)
			})
			_ = keyring.Delete(keyringService, keyringUser)

			// Setup legacy key if needed
			var expectedKey []byte
			if tt.setupLegacy {
				expectedKey = make([]byte, tt.legacyKeySize)
				for i := range expectedKey {
					expectedKey[i] = byte(i)
				}
				legacyPath := filepath.Join(dir, "master.key")
				if err := os.WriteFile(legacyPath, expectedKey, 0600); err != nil {
					t.Fatalf("failed to write legacy key: %v", err)
				}
			}

			// Run migration
			db := database.New(dir)
			migrated, err := m.MigrateFromLegacy(db)

			if tt.shouldFailSave {
				if err == nil {
					t.Error("MigrateFromLegacy() expected error for invalid key size, got none")
					return
				}
				if !migrated {
					return // Expected behavior
				}
			}

			if err != nil {
				t.Fatalf("MigrateFromLegacy() unexpected error = %v", err)
			}

			if migrated != tt.wantMigrated {
				t.Errorf("MigrateFromLegacy() migrated = %v, want %v", migrated, tt.wantMigrated)
			}

			if tt.setupLegacy && tt.legacyKeySize == keySize {
				// Verify key was migrated correctly
				loadedKey, err := m.Load(nil)
				if err != nil {
					t.Fatalf("failed to load migrated key: %v", err)
				}

				if !Equal(expectedKey, loadedKey) {
					t.Error("migrated key does not match original")
				}

				// Verify legacy key file was removed
				legacyPath := filepath.Join(dir, "master.key")
				if _, err := os.Stat(legacyPath); err == nil {
					t.Error("legacy master.key file was not removed after migration")
				}
			}

			// For the invalid key size case, verify that if migration succeeded (to keyring),
			// we can still load it, and the legacy file was removed
			if tt.setupLegacy && tt.legacyKeySize != keySize && migrated {
				// Legacy key file should be removed even if key was wrong size
				legacyPath := filepath.Join(dir, "master.key")
				if _, err := os.Stat(legacyPath); err == nil {
					t.Error("legacy master.key file was not removed even after migration attempt")
				}
			}
		})
	}
}

func TestManagerMigrateFromLegacyWhenKeyExists(t *testing.T) {
	dir := tempDir(t)
	m := New(dir)

	// Create a new key in the new format first
	newKey, err := GenerateKey()
	if err != nil {
		t.Fatalf("GenerateKey() error = %v", err)
	}

	if err := m.Save(newKey); err != nil {
		t.Fatalf("Save() error = %v", err)
	}

	// Create legacy key file and an empty database (no data to migrate)
	legacyKey := make([]byte, keySize)
	for i := range legacyKey {
		legacyKey[i] = byte(i + 1)
	}
	legacyPath := filepath.Join(dir, "master.key")
	if err := os.WriteFile(legacyPath, legacyKey, 0600); err != nil {
		t.Fatalf("failed to write legacy key: %v", err)
	}

	// Run migration - should migrate and remove legacy file
	db := database.New(dir)
	migrated, err := m.MigrateFromLegacy(db)
	if err != nil {
		t.Fatalf("MigrateFromLegacy() unexpected error = %v", err)
	}

	if !migrated {
		t.Error("MigrateFromLegacy() should migrate when legacy key exists")
	}

	// Verify the existing key was not replaced (newer keyring key is kept)
	loadedKey, err := m.Load(nil)
	if err != nil {
		t.Fatalf("failed to load key: %v", err)
	}

	if !Equal(newKey, loadedKey) {
		t.Error("existing key was incorrectly replaced with legacy key")
	}

	// Verify legacy key file was removed
	if _, err := os.Stat(legacyPath); err == nil {
		t.Error("legacy master.key file was not removed after migration")
	}
}
