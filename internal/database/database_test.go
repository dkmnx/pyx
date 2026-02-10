package database

import (
	"os"
	"path/filepath"
	"testing"
)

func TestNewDatabase(t *testing.T) {
	dataDir := t.TempDir()
	db := New(dataDir)

	if db == nil {
		t.Fatal("New() returned nil")
	}

	if db.filePath != filepath.Join(dataDir, "database.json") {
		t.Errorf("New() filePath = %v, want %v", db.filePath, filepath.Join(dataDir, "database.json"))
	}
}

func TestLoadEmpty(t *testing.T) {
	dataDir := t.TempDir()
	db := New(dataDir)

	if err := db.Load(); err != nil {
		t.Fatalf("Load() error = %v", err)
	}

	entries := db.ListEntries()
	if len(entries) != 0 {
		t.Errorf("Load() entries count = %d, want 0", len(entries))
	}
}

func TestSaveLoad(t *testing.T) {
	dataDir := t.TempDir()
	db := New(dataDir)

	entry := NewEntry("test-label", "openai", "cipher", "nonce")
	if err := db.AddEntry(entry); err != nil {
		t.Fatalf("AddEntry() error = %v", err)
	}

	if err := db.Save(); err != nil {
		t.Fatalf("Save() error = %v", err)
	}

	// Verify file exists
	filePath := filepath.Join(dataDir, "database.json")
	if _, err := os.Stat(filePath); os.IsNotExist(err) {
		t.Fatal("Save() did not create database file")
	}

	// Check file permissions
	info, err := os.Stat(filePath)
	if err != nil {
		t.Fatalf("os.Stat() error = %v", err)
	}

	if info.Mode().Perm() != 0600 {
		t.Errorf("Save() file permissions = %v, want 0600", info.Mode().Perm())
	}

	// Load in new database instance
	db2 := New(dataDir)
	if err := db2.Load(); err != nil {
		t.Fatalf("Load() error = %v", err)
	}

	entries := db2.ListEntries()
	if len(entries) != 1 {
		t.Fatalf("Load() entries count = %d, want 1", len(entries))
	}

	if entries[0].Label != "test-label" {
		t.Errorf("Load() entry label = %v, want test-label", entries[0].Label)
	}
}

func TestAddEntry(t *testing.T) {
	dataDir := t.TempDir()
	db := New(dataDir)

	entry1 := NewEntry("label1", "openai", "cipher1", "nonce1")
	if err := db.AddEntry(entry1); err != nil {
		t.Fatalf("AddEntry() error = %v", err)
	}

	entries := db.ListEntries()
	if len(entries) != 1 {
		t.Errorf("AddEntry() entries count = %d, want 1", len(entries))
	}

	entry2 := NewEntry("label2", "anthropic", "cipher2", "nonce2")
	if err := db.AddEntry(entry2); err != nil {
		t.Fatalf("AddEntry() error = %v", err)
	}

	entries = db.ListEntries()
	if len(entries) != 2 {
		t.Errorf("AddEntry() entries count = %d, want 2", len(entries))
	}
}

func TestAddEntryDuplicateLabel(t *testing.T) {
	dataDir := t.TempDir()
	db := New(dataDir)

	entry1 := NewEntry("label1", "openai", "cipher1", "nonce1")
	if err := db.AddEntry(entry1); err != nil {
		t.Fatalf("AddEntry() error = %v", err)
	}

	entry2 := NewEntry("label1", "anthropic", "cipher2", "nonce2")
	err := db.AddEntry(entry2)
	if err != ErrDuplicateLabel {
		t.Errorf("AddEntry() error = %v, want %v", err, ErrDuplicateLabel)
	}
}

func TestGetEntry(t *testing.T) {
	dataDir := t.TempDir()
	db := New(dataDir)

	entry := NewEntry("test-label", "openai", "cipher", "nonce")
	if err := db.AddEntry(entry); err != nil {
		t.Fatalf("AddEntry() error = %v", err)
	}

	retrieved, err := db.GetEntry(entry.ID)
	if err != nil {
		t.Fatalf("GetEntry() error = %v", err)
	}

	if retrieved.Label != "test-label" {
		t.Errorf("GetEntry() label = %v, want test-label", retrieved.Label)
	}

	_, err = db.GetEntry("non-existent-id")
	if err != ErrEntryNotFound {
		t.Errorf("GetEntry() error = %v, want %v", err, ErrEntryNotFound)
	}
}

func TestGetEntryByLabel(t *testing.T) {
	dataDir := t.TempDir()
	db := New(dataDir)

	entry := NewEntry("test-label", "openai", "cipher", "nonce")
	if err := db.AddEntry(entry); err != nil {
		t.Fatalf("AddEntry() error = %v", err)
	}

	retrieved, err := db.GetEntryByLabel("test-label")
	if err != nil {
		t.Fatalf("GetEntryByLabel() error = %v", err)
	}

	if retrieved.ID != entry.ID {
		t.Errorf("GetEntryByLabel() ID = %v, want %v", retrieved.ID, entry.ID)
	}

	_, err = db.GetEntryByLabel("non-existent-label")
	if err != ErrEntryNotFound {
		t.Errorf("GetEntryByLabel() error = %v, want %v", err, ErrEntryNotFound)
	}
}

func TestDeleteEntry(t *testing.T) {
	dataDir := t.TempDir()
	db := New(dataDir)

	entry := NewEntry("test-label", "openai", "cipher", "nonce")
	if err := db.AddEntry(entry); err != nil {
		t.Fatalf("AddEntry() error = %v", err)
	}

	if err := db.DeleteEntry(entry.ID); err != nil {
		t.Fatalf("DeleteEntry() error = %v", err)
	}

	entries := db.ListEntries()
	if len(entries) != 0 {
		t.Errorf("DeleteEntry() entries count = %d, want 0", len(entries))
	}

	err := db.DeleteEntry("non-existent-id")
	if err != ErrEntryNotFound {
		t.Errorf("DeleteEntry() error = %v, want %v", err, ErrEntryNotFound)
	}
}

func TestNewEntry(t *testing.T) {
	entry := NewEntry("label", "provider", "cipher", "nonce")

	if entry.ID == "" {
		t.Error("NewEntry() ID is empty")
	}

	if entry.Label != "label" {
		t.Errorf("NewEntry() Label = %v, want label", entry.Label)
	}

	if entry.Provider != "provider" {
		t.Errorf("NewEntry() Provider = %v, want provider", entry.Provider)
	}

	if entry.Cipher != "cipher" {
		t.Errorf("NewEntry() Cipher = %v, want cipher", entry.Cipher)
	}

	if entry.Nonce != "nonce" {
		t.Errorf("NewEntry() Nonce = %v, want nonce", entry.Nonce)
	}

	if entry.CreatedAt.IsZero() {
		t.Error("NewEntry() CreatedAt is zero")
	}
}
