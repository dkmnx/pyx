package cmd

import (
	"context"
	"strings"
	"testing"

	"github.com/dkmnx/ply/internal/crypto"
	"github.com/dkmnx/ply/internal/database"
	"github.com/dkmnx/ply/internal/keys"
)

func TestProviderValidationIntegration(t *testing.T) {
	// Create temporary directory for testing
	tmpDir := t.TempDir()

	// Set up test environment
	ctx := context.Background()
	dataDir := tmpDir

	keyMgr := keys.New(dataDir)
	db := database.New(dataDir)

	// Initialize master key
	masterKey, err := keys.GenerateKey()
	if err != nil {
		t.Fatalf("Failed to generate master key: %v", err)
	}
	defer func() {
		for i := range masterKey {
			masterKey[i] = 0
		}
	}()

	if err := keyMgr.Save(masterKey); err != nil {
		t.Fatalf("Failed to save master key: %v", err)
	}

	// Load database
	if err := db.Load(ctx); err != nil {
		t.Fatalf("Failed to load database: %v", err)
	}

	// Add a valid provider
	validProvider := "anthropic"
	apiKey := "sk-test-key-12345"
	cipher, nonce, err := crypto.Encrypt(masterKey, apiKey)
	if err != nil {
		t.Fatalf("Failed to encrypt API key: %v", err)
	}

	entry := database.NewEntry(validProvider, cipher, nonce)
	if err := db.AddEntry(entry); err != nil {
		t.Fatalf("Failed to add entry: %v", err)
	}
	if err := db.Save(ctx); err != nil {
		t.Fatalf("Failed to save database: %v", err)
	}

	// Test 1: Valid provider can be resolved
	t.Run("valid provider resolves", func(t *testing.T) {
		entries, err := resolveEntries(db, validProvider)
		if err != nil {
			t.Errorf("resolveEntries(%q) error = %v, want nil", validProvider, err)
		}
		if len(entries) != 1 {
			t.Errorf("resolveEntries(%q) returned %d entries, want 1", validProvider, len(entries))
		}
		if entries[0].Provider != validProvider {
			t.Errorf("resolveEntries(%q) returned provider %q, want %q", validProvider, entries[0].Provider, validProvider)
		}
	})

	// Test 2: Provider with invalid format is rejected
	t.Run("invalid provider format rejected", func(t *testing.T) {
		invalidProvider := "anthropic@bad"
		_, err := resolveEntries(db, invalidProvider)
		if err == nil {
			t.Error("resolveEntries(" + invalidProvider + ") returned nil error, want error")
		}
		if !strings.Contains(err.Error(), "must be 1-50 characters") {
			t.Errorf("resolveEntries(%q) error = %v, should contain format error", invalidProvider, err)
		}
	})

	// Test 3: Provider with path traversal is rejected
	t.Run("path traversal rejected", func(t *testing.T) {
		pathTraversal := "../etc/passwd"
		_, err := resolveEntries(db, pathTraversal)
		if err == nil {
			t.Error("resolveEntries(" + pathTraversal + ") returned nil error, want error")
		}
		if !strings.Contains(err.Error(), "path traversal") {
			t.Errorf("resolveEntries(%q) error = %v, should contain path traversal error", pathTraversal, err)
		}
	})

	// Test 4: Empty provider returns all entries (no validation for empty)
	t.Run("empty provider returns all entries", func(t *testing.T) {
		entries, err := resolveEntries(db, "")
		if err != nil {
			t.Errorf("resolveEntries(\"\") returned error: %v, want nil (returns all entries)", err)
		}
		if len(entries) == 0 {
			t.Error("resolveEntries(\"\") returned no entries, want all configured entries")
		}
	})
}
