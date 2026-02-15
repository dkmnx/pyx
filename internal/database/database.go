// Package database manages the persistent storage of encrypted API keys.
package database

import (
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"sync"
	"time"

	"github.com/google/uuid"
)

var (
	// ErrEntryNotFound is returned when an entry with the given ID is not found.
	ErrEntryNotFound = errors.New("entry not found")

	// ErrDuplicateLabel is returned when an entry with the same label already exists.
	ErrDuplicateLabel = errors.New("duplicate label")
)

// Entry represents a stored encrypted API key entry.
type Entry struct {
	ID           string    `json:"id"`
	Label        string    `json:"label"`
	Provider     string    `json:"provider"`
	DefaultModel string    `json:"default_model,omitempty"`
	Cipher       string    `json:"cipher"`
	Nonce        string    `json:"nonce"`
	CreatedAt    time.Time `json:"created_at"`
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
func (db *Database) Load() error {
	db.mu.Lock()
	defer db.mu.Unlock()

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

	return nil
}

// Save writes entries to the database file.
func (db *Database) Save() error {
	db.mu.RLock()
	defer db.mu.RUnlock()

	data, err := json.MarshalIndent(db.entries, "", "  ")
	if err != nil {
		return fmt.Errorf("failed to marshal database: %w", err)
	}

	// Ensure parent directory exists
	if err := os.MkdirAll(filepath.Dir(db.filePath), 0700); err != nil {
		return fmt.Errorf("failed to create data directory: %w", err)
	}

	if err := os.WriteFile(db.filePath, data, 0600); err != nil {
		return fmt.Errorf("failed to write database file: %w", err)
	}

	return nil
}

// AddEntry adds a new entry to the database.
func (db *Database) AddEntry(entry Entry) error {
	db.mu.Lock()
	defer db.mu.Unlock()

	// Check for duplicate label
	for _, e := range db.entries {
		if e.Label == entry.Label {
			return ErrDuplicateLabel
		}
	}

	db.entries = append(db.entries, entry)
	return nil
}

// GetEntry retrieves an entry by ID.
func (db *Database) GetEntry(id string) (Entry, error) {
	db.mu.RLock()
	defer db.mu.RUnlock()

	for _, e := range db.entries {
		if e.ID == id {
			return e, nil
		}
	}

	return Entry{}, ErrEntryNotFound
}

// GetEntryByLabel retrieves an entry by label.
func (db *Database) GetEntryByLabel(label string) (Entry, error) {
	db.mu.RLock()
	defer db.mu.RUnlock()

	for _, e := range db.entries {
		if e.Label == label {
			return e, nil
		}
	}

	return Entry{}, ErrEntryNotFound
}

// GetEntryByLabelOrID retrieves an entry by label or ID.
// First tries to find by label, then falls back to ID lookup.
func (db *Database) GetEntryByLabelOrID(target string) (Entry, error) {
	entry, err := db.GetEntryByLabel(target)
	if err == nil {
		return entry, nil
	}
	return db.GetEntry(target)
}

// ListEntries returns all entries.
func (db *Database) ListEntries() []Entry {
	db.mu.RLock()
	defer db.mu.RUnlock()

	result := make([]Entry, len(db.entries))
	copy(result, db.entries)
	return result
}

// DeleteEntry removes an entry by ID.
func (db *Database) DeleteEntry(id string) error {
	db.mu.Lock()
	defer db.mu.Unlock()

	for i, e := range db.entries {
		if e.ID == id {
			db.entries = append(db.entries[:i], db.entries[i+1:]...)
			return nil
		}
	}

	return ErrEntryNotFound
}

// UpdateEntry updates an existing entry in the database.
// It updates the entry with the same ID as the provided entry.
// Returns ErrEntryNotFound if no entry with the ID exists.
// If the new label conflicts with another entry, returns ErrDuplicateLabel.
func (db *Database) UpdateEntry(entry Entry) error {
	db.mu.Lock()
	defer db.mu.Unlock()

	for i, e := range db.entries {
		if e.ID == entry.ID {
			for j, other := range db.entries {
				if i != j && other.Label == entry.Label {
					return ErrDuplicateLabel
				}
			}
			db.entries[i] = entry
			return nil
		}
	}

	return ErrEntryNotFound
}

// NewEntry creates a new Entry with a generated ID and timestamp.
func NewEntry(label, provider, cipher, nonce string) Entry {
	return Entry{
		ID:        uuid.New().String(),
		Label:     label,
		Provider:  provider,
		Cipher:    cipher,
		Nonce:     nonce,
		CreatedAt: time.Now().UTC(),
	}
}
