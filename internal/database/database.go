// Package database manages the persistent storage of encrypted API keys.
package database

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"sort"
	"sync"
	"time"
)

var (
	ErrEntryNotFound     = errors.New("entry not found")
	ErrDuplicateProvider = errors.New("duplicate provider")
	ErrBackupFailed      = errors.New("failed to create backup")
)

// Entry represents a stored encrypted API key entry.
type Entry struct {
	Provider  string    `json:"provider"`
	Cipher    string    `json:"cipher"`
	Nonce     string    `json:"nonce"`
	CreatedAt time.Time `json:"created_at"`
	UpdatedAt time.Time `json:"updated_at,omitempty"`
}

// Database manages the encrypted API key storage.
type Database struct {
	filePath string
	entries  []Entry
	mu       sync.RWMutex
}

// New creates a new Database instance.
func New(dataDir string) *Database {
	return &Database{
		filePath: filepath.Join(dataDir, "database.json"),
		entries:  make([]Entry, 0),
	}
}

// Load loads entries from the database file.
// If the file doesn't exist, it initializes an empty database.
func (db *Database) Load(ctx context.Context) error {
	db.mu.Lock()
	defer db.mu.Unlock()

	select {
	case <-ctx.Done():
		return ctx.Err()
	default:
	}

	data, err := os.ReadFile(db.filePath)
	if err != nil {
		if os.IsNotExist(err) {
			db.entries = make([]Entry, 0)
			return nil
		}
		return fmt.Errorf("failed to read database file: %w", err)
	}

	if err := json.Unmarshal(data, &db.entries); err != nil {
		return fmt.Errorf("failed to unmarshal database: %w", err)
	}

	if warnings := normalizeEntries(db.filePath, &db.entries); len(warnings) > 0 {
		for _, warning := range warnings {
			fmt.Fprintln(os.Stderr, warning)
		}
	}

	return nil
}

// Save writes entries to the database file with atomic write and backup.
func (db *Database) Save(ctx context.Context) error {
	db.mu.Lock()
	defer db.mu.Unlock()

	select {
	case <-ctx.Done():
		return ctx.Err()
	default:
	}

	data, err := json.MarshalIndent(db.entries, "", "  ")
	if err != nil {
		return fmt.Errorf("failed to marshal database: %w", err)
	}

	if err := os.MkdirAll(filepath.Dir(db.filePath), 0700); err != nil {
		return fmt.Errorf("failed to create data directory: %w", err)
	}

	backupPath := db.filePath + ".bak"
	if _, err := os.Stat(db.filePath); err == nil {
		if err := copyFile(db.filePath, backupPath); err != nil {
			return fmt.Errorf("%w: %v", ErrBackupFailed, err)
		}
	}

	tempPath := db.filePath + ".tmp"
	if err := os.WriteFile(tempPath, data, 0600); err != nil {
		return fmt.Errorf("failed to write temp database file: %w", err)
	}

	if err := os.Rename(tempPath, db.filePath); err != nil {
		return fmt.Errorf("failed to rename database file: %w", err)
	}

	return nil
}

func copyFile(src, dst string) error {
	data, err := os.ReadFile(src)
	if err != nil {
		return err
	}
	return os.WriteFile(dst, data, 0600)
}

// AddEntry adds a new entry to the database.
func (db *Database) AddEntry(entry Entry) error {
	db.mu.Lock()
	defer db.mu.Unlock()

	// Check for duplicate provider
	for _, e := range db.entries {
		if e.Provider == entry.Provider {
			return ErrDuplicateProvider
		}
	}

	db.entries = append(db.entries, entry)
	return nil
}

// GetEntry retrieves an entry by provider.
func (db *Database) GetEntry(id string) (Entry, error) {
	db.mu.RLock()
	defer db.mu.RUnlock()

	for _, e := range db.entries {
		if e.Provider == id {
			return e, nil
		}
	}

	return Entry{}, ErrEntryNotFound
}

// ListEntries returns all entries.
func (db *Database) ListEntries() []Entry {
	db.mu.RLock()
	defer db.mu.RUnlock()

	result := make([]Entry, len(db.entries))
	copy(result, db.entries)
	return result
}

// DeleteEntry removes an entry by provider.
func (db *Database) DeleteEntry(id string) error {
	db.mu.Lock()
	defer db.mu.Unlock()

	for i, e := range db.entries {
		if e.Provider == id {
			db.entries = append(db.entries[:i], db.entries[i+1:]...)
			return nil
		}
	}

	return ErrEntryNotFound
}

// UpdateEntry updates an existing entry in the database.
// It updates the entry with the same provider as the provided entry.
// Returns ErrEntryNotFound if no entry with the provider exists.
func (db *Database) UpdateEntry(entry Entry) error {
	db.mu.Lock()
	defer db.mu.Unlock()

	for i, e := range db.entries {
		if e.Provider == entry.Provider {
			db.entries[i] = entry
			return nil
		}
	}

	return ErrEntryNotFound
}

// newerEntry compares two entries and returns true if a is newer than b.
func newerEntry(a, b Entry) bool {
	aTime := a.UpdatedAt
	if aTime.IsZero() {
		aTime = a.CreatedAt
	}
	bTime := b.UpdatedAt
	if bTime.IsZero() {
		bTime = b.CreatedAt
	}
	return aTime.After(bTime)
}

// duplicateDetector tracks duplicate providers and finds newest entries.
type duplicateDetector struct {
	byProvider map[string]Entry
	counts     map[string]int
	dropped    map[string][]Entry
}

// newDuplicateDetector creates a new duplicate detector.
func newDuplicateDetector() *duplicateDetector {
	return &duplicateDetector{
		byProvider: make(map[string]Entry),
		counts:    make(map[string]int),
		dropped:   make(map[string][]Entry),
	}
}

// processEntry adds an entry to the detector and tracks duplicates.
// Returns true if an entry was dropped (replaced by newer), false if kept.
func (d *duplicateDetector) processEntry(entry Entry) bool {
	if entry.Provider == "" {
		return false
	}

	existing, ok := d.byProvider[entry.Provider]
	if !ok {
		d.byProvider[entry.Provider] = entry
		d.counts[entry.Provider] = 1
		return false
	}

	d.counts[entry.Provider]++
	if newerEntry(entry, existing) {
		d.dropped[entry.Provider] = append(d.dropped[entry.Provider], existing)
		d.byProvider[entry.Provider] = entry
		return true // Dropped old
	}

	d.dropped[entry.Provider] = append(d.dropped[entry.Provider], entry)
	return true // Dropped current
}

// hasDuplicates returns true if any duplicates were found.
func (d *duplicateDetector) hasDuplicates() bool {
	for _, v := range d.counts {
		if v > 1 {
			return true
		}
	}
	return false
}

// createBackup creates a backup of dropped entries.
func createBackup(dataDir string, dropped map[string][]Entry) error {
	if len(dropped) == 0 {
		return nil
	}

	backupData := make([]Entry, 0)
	for _, entries := range dropped {
		backupData = append(backupData, entries...)
	}

	if len(backupData) == 0 {
		return nil
	}

	backupPath := filepath.Join(dataDir, "database.json.dropped.bak")
	backupJSON, err := json.MarshalIndent(backupData, "", "  ")
	if err != nil {
		return fmt.Errorf("failed to marshal backup: %w", err)
	}

	if err := os.WriteFile(backupPath, backupJSON, 0600); err != nil {
		return fmt.Errorf("failed to write backup: %w", err)
	}

	return nil
}

func normalizeEntries(filePath string, entries *[]Entry) []string {
	if entries == nil || len(*entries) == 0 {
		return nil
	}

	detector := newDuplicateDetector()

	// Process all entries to find duplicates
	for _, entry := range *entries {
		detector.processEntry(entry)
	}

	// If no duplicates, nothing to do
	if !detector.hasDuplicates() {
		return nil
	}

	// Create backup of dropped entries
	dataDir := filepath.Dir(filePath)
	if err := createBackup(dataDir, detector.dropped); err != nil {
		return []string{fmt.Sprintf("Warning: %v", err)}
	}

	// Update entries list to only keep unique entries
	providers := make([]string, 0, len(detector.byProvider))
	for provider := range detector.byProvider {
		providers = append(providers, provider)
	}
	sort.Strings(providers)

	normalized := make([]Entry, 0, len(detector.byProvider))
	warnings := make([]string, 0)
	for _, provider := range providers {
		entry := detector.byProvider[provider]
		normalized = append(normalized, entry)
		if detector.counts[provider] > 1 {
			warnings = append(warnings, fmt.Sprintf("Warning: multiple entries for provider '%s' found; keeping newest and dropping %d old entries. Dropped entries backed up to database.json.dropped.bak", provider, detector.counts[provider]-1))
		}
	}

	*entries = normalized
	return warnings
}

// NewEntry creates a new Entry with generated timestamps.
func NewEntry(provider, cipher, nonce string) Entry {
	now := time.Now().UTC()
	return Entry{
		Provider:  provider,
		Cipher:    cipher,
		Nonce:     nonce,
		CreatedAt: now,
		UpdatedAt: now,
	}
}
