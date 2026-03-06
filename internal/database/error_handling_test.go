package database

import (
	"context"
	"os"
	"path/filepath"
	"runtime"
	"strings"
	"testing"
	"time"
)

func TestLoadContextCancellation(t *testing.T) {
	dataDir := t.TempDir()
	db := New(dataDir)

	// Create a context that's already cancelled
	ctx, cancel := context.WithCancel(context.Background())
	cancel()

	err := db.Load(ctx)
	if err != context.Canceled {
		t.Errorf("Load() with cancelled context error = %v, want %v", err, context.Canceled)
	}
}

func TestSaveContextCancellation(t *testing.T) {
	dataDir := t.TempDir()
	db := New(dataDir)

	entry := NewEntry("openai", "cipher")
	if err := db.AddEntry(entry); err != nil {
		t.Fatalf("AddEntry() error = %v", err)
	}

	// Cancel context before save
	ctx, cancel := context.WithCancel(context.Background())
	cancel()

	err := db.Save(ctx)
	if err != context.Canceled {
		t.Errorf("Save() with cancelled context error = %v, want %v", err, context.Canceled)
	}
}

func TestSaveContextCancellationDuringBackup(t *testing.T) {
	dataDir := t.TempDir()
	db := New(dataDir)

	entry := NewEntry("openai", "cipher")
	if err := db.AddEntry(entry); err != nil {
		t.Fatalf("AddEntry() error = %v", err)
	}

	// Save once to create the initial file
	if err := db.Save(context.Background()); err != nil {
		t.Fatalf("Save() error = %v", err)
	}

	// Add another entry
	entry2 := NewEntry("anthropic", "cipher2")
	if err := db.AddEntry(entry2); err != nil {
		t.Fatalf("AddEntry() error = %v", err)
	}

	// Cancel context during second save (when backup would be created)
	ctx, cancel := context.WithCancel(context.Background())
	cancel()

	err := db.Save(ctx)
	if err != context.Canceled {
		t.Errorf("Save() with cancelled context during backup error = %v, want %v", err, context.Canceled)
	}
}

func TestUpdateEntryNotFound(t *testing.T) {
	dataDir := t.TempDir()
	db := New(dataDir)

	entry := NewEntry("openai", "cipher")
	err := db.UpdateEntry(entry)
	if err != ErrEntryNotFound {
		t.Errorf("UpdateEntry() error = %v, want ErrEntryNotFound", err)
	}
}

func TestUpdateExistingEntry(t *testing.T) {
	dataDir := t.TempDir()
	db := New(dataDir)

	entry := NewEntry("openai", "cipher1")
	if err := db.AddEntry(entry); err != nil {
		t.Fatalf("AddEntry() error = %v", err)
	}

	// Update the entry
	updatedEntry := NewEntry("openai", "cipher2")
	if err := db.UpdateEntry(updatedEntry); err != nil {
		t.Fatalf("UpdateEntry() error = %v", err)
	}

	// Verify the update
	retrieved, err := db.GetEntry("openai")
	if err != nil {
		t.Fatalf("GetEntry() error = %v", err)
	}

	if retrieved.Cipher != "cipher2" {
		t.Errorf("UpdateEntry() cipher = %v, want cipher2", retrieved.Cipher)
	}
}

func TestNormalizeNilEntries(t *testing.T) {
	warnings := normalizeEntries("/test/path.json", nil)
	if warnings != nil {
		t.Errorf("normalizeEntries() with nil entries should return nil warnings, got %v", warnings)
	}
}

func TestNormalizeEmptyEntries(t *testing.T) {
	entries := make([]Entry, 0)
	warnings := normalizeEntries("/test/path.json", &entries)
	if warnings != nil {
		t.Errorf("normalizeEntries() with empty entries should return nil warnings, got %v", warnings)
	}
}

func TestCreateBackupWriteError(t *testing.T) {
	// Create a directory structure that will fail
	if runtime.GOOS == "windows" {
		t.Skip("Skipping on Windows due to different permission model")
	}

	tempDir := t.TempDir()

	// Create a file where we want to create a directory
	badPath := filepath.Join(tempDir, "bad-dir")
	if err := os.WriteFile(badPath, []byte("file"), 0600); err != nil {
		t.Fatalf("Failed to create file: %v", err)
	}

	// Try to create backup in the file path (should fail)
	dropped := map[string][]Entry{
		"openai": {{Provider: "openai", Cipher: "old"}},
	}

	// This should fail because we're trying to write to a file path
	err := createBackup(badPath, dropped)
	if err == nil {
		t.Error("createBackup() should fail when writing to invalid path")
	}
}

func TestLoadInvalidJSON(t *testing.T) {
	dataDir := t.TempDir()
	db := New(dataDir)

	// Write invalid JSON to the database file
	filePath := filepath.Join(dataDir, "database.json")
	if err := os.WriteFile(filePath, []byte("invalid json{{{"), 0600); err != nil {
		t.Fatalf("Failed to write file: %v", err)
	}

	err := db.Load(context.Background())
	if err == nil {
		t.Error("Load() with invalid JSON should return error")
	}

	if !strings.Contains(err.Error(), "failed to unmarshal") {
		t.Errorf("Load() error should mention unmarshal failure, got %v", err)
	}
}

func TestLoadNonExistentFile(t *testing.T) {
	dataDir := t.TempDir()
	db := New(dataDir)

	// Don't create the file - Load should handle this gracefully
	err := db.Load(context.Background())
	if err != nil {
		t.Errorf("Load() with non-existent file should not error, got %v", err)
	}

	entries := db.ListEntries()
	if len(entries) != 0 {
		t.Errorf("Load() should initialize empty entries, got %d entries", len(entries))
	}
}

func TestSavePermissionError(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("Skipping on Windows due to different permission model")
	}

	dataDir := t.TempDir()
	db := New(dataDir)

	entry := NewEntry("openai", "cipher")
	if err := db.AddEntry(entry); err != nil {
		t.Fatalf("AddEntry() error = %v", err)
	}

	// Make the directory unreadable
	if err := os.Chmod(dataDir, 0000); err != nil {
		t.Fatalf("Failed to chmod: %v", err)
	}
	defer os.Chmod(dataDir, 0700)

	err := db.Save(context.Background())
	if err == nil {
		t.Error("Save() should fail when directory is not writable")
	}
}

func TestDeleteEntryThenSave(t *testing.T) {
	dataDir := t.TempDir()
	db := New(dataDir)

	// Add and save an entry
	entry := NewEntry("openai", "cipher")
	if err := db.AddEntry(entry); err != nil {
		t.Fatalf("AddEntry() error = %v", err)
	}

	if err := db.Save(context.Background()); err != nil {
		t.Fatalf("Save() error = %v", err)
	}

	// Delete the entry
	if err := db.DeleteEntry("openai"); err != nil {
		t.Fatalf("DeleteEntry() error = %v", err)
	}

	// Save again
	if err := db.Save(context.Background()); err != nil {
		t.Fatalf("Save() after delete error = %v", err)
	}

	// Verify file exists and is valid
	if err := db.Load(context.Background()); err != nil {
		t.Fatalf("Load() error = %v", err)
	}

	entries := db.ListEntries()
	if len(entries) != 0 {
		t.Errorf("Expected 0 entries after delete, got %d", len(entries))
	}
}

func TestAddMultipleEntries(t *testing.T) {
	dataDir := t.TempDir()
	db := New(dataDir)

	// Add multiple entries
	providers := []string{"openai", "anthropic", "google", "groq"}
	for _, provider := range providers {
		entry := NewEntry(provider, "cipher-"+provider)
		if err := db.AddEntry(entry); err != nil {
			t.Fatalf("AddEntry() error for %s = %v", provider, err)
		}
	}

	entries := db.ListEntries()
	if len(entries) != 4 {
		t.Errorf("Expected 4 entries, got %d", len(entries))
	}
}

func TestGetEntryAfterUpdate(t *testing.T) {
	dataDir := t.TempDir()
	db := New(dataDir)

	// Add entry
	entry := NewEntry("openai", "cipher1")
	if err := db.AddEntry(entry); err != nil {
		t.Fatalf("AddEntry() error = %v", err)
	}

	// Update entry
	updatedEntry := Entry{
		Provider:  "openai",
		Cipher:    "cipher2",
		CreatedAt: time.Now().UTC(),
		UpdatedAt: time.Now().UTC().Add(time.Hour),
	}
	if err := db.UpdateEntry(updatedEntry); err != nil {
		t.Fatalf("UpdateEntry() error = %v", err)
	}

	// Get and verify
	retrieved, err := db.GetEntry("openai")
	if err != nil {
		t.Fatalf("GetEntry() error = %v", err)
	}

	if retrieved.Cipher != "cipher2" {
		t.Errorf("GetEntry() cipher = %v, want cipher2", retrieved.Cipher)
	}

	// UpdatedAt should be preserved
	if retrieved.UpdatedAt.IsZero() {
		t.Error("GetEntry() UpdatedAt should not be zero after update")
	}
}

func TestDeleteAllEntries(t *testing.T) {
	dataDir := t.TempDir()
	db := New(dataDir)

	// Add multiple entries
	providers := []string{"openai", "anthropic", "google"}
	for _, provider := range providers {
		entry := NewEntry(provider, "cipher-"+provider)
		if err := db.AddEntry(entry); err != nil {
			t.Fatalf("AddEntry() error = %v", err)
		}
	}

	// Delete all entries
	for _, provider := range providers {
		if err := db.DeleteEntry(provider); err != nil {
			t.Fatalf("DeleteEntry() error = %v", err)
		}
	}

	entries := db.ListEntries()
	if len(entries) != 0 {
		t.Errorf("Expected 0 entries after deleting all, got %d", len(entries))
	}
}

func TestListEntriesIsCopy(t *testing.T) {
	dataDir := t.TempDir()
	db := New(dataDir)

	entry := NewEntry("openai", "cipher")
	if err := db.AddEntry(entry); err != nil {
		t.Fatalf("AddEntry() error = %v", err)
	}

	// Get the list
	list1 := db.ListEntries()

	// Modify the list
	if len(list1) > 0 {
		list1[0].Cipher = "modified"
	}

	// Get the list again
	list2 := db.ListEntries()

	// The second list should not be modified
	if len(list2) > 0 && list2[0].Cipher == "modified" {
		t.Error("ListEntries() should return a copy, not the original slice")
	}
}

func TestNewerEntryComparison(t *testing.T) {
	tests := []struct {
		name   string
		first  Entry
		second Entry
		want   bool
	}{
		{
			name: "first newer by UpdatedAt",
			first: Entry{
				UpdatedAt: time.Date(2024, time.January, 2, 0, 0, 0, 0, time.UTC),
			},
			second: Entry{
				UpdatedAt: time.Date(2024, time.January, 1, 0, 0, 0, 0, time.UTC),
			},
			want: true,
		},
		{
			name: "first older by UpdatedAt",
			first: Entry{
				UpdatedAt: time.Date(2024, time.January, 1, 0, 0, 0, 0, time.UTC),
			},
			second: Entry{
				UpdatedAt: time.Date(2024, time.January, 2, 0, 0, 0, 0, time.UTC),
			},
			want: false,
		},
		{
			name: "first uses CreatedAt when UpdatedAt is zero",
			first: Entry{
				CreatedAt: time.Date(2024, time.January, 2, 0, 0, 0, 0, time.UTC),
			},
			second: Entry{
				CreatedAt: time.Date(2024, time.January, 1, 0, 0, 0, 0, time.UTC),
			},
			want: true,
		},
		{
			name: "both zero times",
			first: Entry{
				CreatedAt: time.Time{},
				UpdatedAt: time.Time{},
			},
			second: Entry{
				CreatedAt: time.Time{},
				UpdatedAt: time.Time{},
			},
			want: false,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := newerEntry(tt.first, tt.second)
			if got != tt.want {
				t.Errorf("newerEntry() = %v, want %v", got, tt.want)
			}
		})
	}
}

func TestDuplicateDetectorNoDuplicates(t *testing.T) {
	detector := newDuplicateDetector()

	entries := []Entry{
		{Provider: "openai", UpdatedAt: time.Date(2024, time.January, 1, 0, 0, 0, 0, time.UTC)},
		{Provider: "anthropic", UpdatedAt: time.Date(2024, time.January, 1, 0, 0, 0, 0, time.UTC)},
		{Provider: "google", UpdatedAt: time.Date(2024, time.January, 1, 0, 0, 0, 0, time.UTC)},
	}

	for _, entry := range entries {
		dropped := detector.processEntry(entry)
		if dropped {
			t.Errorf("Entry should not be dropped when no duplicates: %s", entry.Provider)
		}
	}

	if detector.hasDuplicates() {
		t.Error("Should not detect duplicates when all providers are unique")
	}
}

func TestNormalizeEntriesWithDuplicates(t *testing.T) {
	tempDir := t.TempDir()
	filePath := filepath.Join(tempDir, "database.json")

	entries := []Entry{
		{Provider: "openai", Cipher: "old", UpdatedAt: time.Date(2024, time.January, 1, 0, 0, 0, 0, time.UTC)},
		{Provider: "openai", Cipher: "new", UpdatedAt: time.Date(2024, time.January, 2, 0, 0, 0, 0, time.UTC)},
	}

	warnings := normalizeEntries(filePath, &entries)

	// Should have kept only the newer entry
	if len(entries) != 1 {
		t.Errorf("Expected 1 entry after normalization, got %d", len(entries))
	}

	if entries[0].Cipher != "new" {
		t.Errorf("Expected newer entry to be kept, got cipher = %v", entries[0].Cipher)
	}

	// Should have generated a warning
	if len(warnings) == 0 {
		t.Error("Expected warning about duplicate entries")
	}

	// Backup file should have been created
	backupPath := filepath.Join(tempDir, "database.json.dropped.bak")
	if _, err := os.Stat(backupPath); err != nil {
		t.Errorf("Backup file should be created, got error: %v", err)
	}
}

func TestCreateBackupEmptyDropped(t *testing.T) {
	tempDir := t.TempDir()

	// Empty dropped map should not create a backup
	dropped := map[string][]Entry{}
	err := createBackup(tempDir, dropped)
	if err != nil {
		t.Errorf("createBackup() with empty dropped should not error, got %v", err)
	}

	// Verify no backup file was created
	backupPath := filepath.Join(tempDir, "database.json.dropped.bak")
	if _, err := os.Stat(backupPath); err == nil {
		t.Error("Backup file should not be created for empty dropped map")
	}
}
