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

func normalizeEntries(filePath string, entries *[]Entry) []string {
	if entries == nil || len(*entries) == 0 {
		return nil
	}

	byProvider := make(map[string]Entry)
	counts := make(map[string]int)
	dropped := make(map[string][]Entry)

	for _, entry := range *entries {
		if entry.Provider == "" {
			continue
		}
		if existing, ok := byProvider[entry.Provider]; ok {
			counts[entry.Provider]++
			if newerEntry(entry, existing) {
				dropped[entry.Provider] = append(dropped[entry.Provider], byProvider[entry.Provider])
				byProvider[entry.Provider] = entry
			} else {
				dropped[entry.Provider] = append(dropped[entry.Provider], entry)
			}
			continue
		}
		byProvider[entry.Provider] = entry
		counts[entry.Provider] = 1
	}

	hasDropped := false
	for _, v := range counts {
		if v > 1 {
			hasDropped = true
			break
		}
	}

	if hasDropped && len(*entries) > 0 {
		backupData := make([]Entry, 0)
		for _, entries := range dropped {
			backupData = append(backupData, entries...)
		}
		if len(backupData) > 0 {
			backupPath := filepath.Join(filepath.Dir(filePath), "database.json.dropped.bak")
			if backupJSON, err := json.MarshalIndent(backupData, "", "  "); err == nil {
				if err := os.WriteFile(backupPath, backupJSON, 0600); err != nil {
					fmt.Fprintf(os.Stderr, "Warning: failed to write backup file: %v\n", err)
				}
			}
		}
	}

	providers := make([]string, 0, len(byProvider))
	for provider := range byProvider {
		providers = append(providers, provider)
	}
	sort.Strings(providers)

	normalized := make([]Entry, 0, len(byProvider))
	warnings := make([]string, 0)
	for _, provider := range providers {
		entry := byProvider[provider]
		normalized = append(normalized, entry)
		if counts[provider] > 1 {
			warnings = append(warnings, fmt.Sprintf("Warning: multiple entries for provider '%s' found; keeping newest and dropping %d old entries. Dropped entries backed up to database.json.dropped.bak", provider, counts[provider]-1))
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
