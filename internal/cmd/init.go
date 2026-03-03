package cmd

import (
	"context"
	"fmt"
	"strings"
	"time"

	"github.com/dkmnx/ply/internal/crypto"
	"github.com/dkmnx/ply/internal/database"
	"github.com/dkmnx/ply/internal/fs"
	"github.com/dkmnx/ply/internal/keys"
	"github.com/dkmnx/ply/internal/prompt"
	"github.com/spf13/cobra"
	"github.com/yarlson/tap"
)

var initCmd = &cobra.Command{
	Use:   "init",
	Short: "Initialize ply master key",
	Long:  `Init creates the master encryption key if it doesn't already exist.`,
	Run:   runInit,
}

func init() {
	rootCmd.AddCommand(initCmd)
}

func databaseExists(ctx context.Context, db *database.Database) (bool, error) {
	if err := db.Load(ctx); err != nil {
		return false, err
	}
	entries := db.ListEntries()
	return len(entries) > 0, nil
}

func runInit(cmd *cobra.Command, args []string) {
	ctx := context.Background()

	fmt.Println()

	tap.Intro("Ply Initialization")

	dataDir, err := fs.EnsureDataDir()
	if err != nil {
		tap.Cancel("Error creating data directory")
		return
	}

	keyMgr := keys.New(dataDir)
	db := database.New(dataDir)

	keyExists, err := keyMgr.Exists()
	if err != nil {
		tap.Cancel("Error checking master key")
		return
	}

	hasDatabase, err := databaseExists(ctx, db)
	if err != nil {
		tap.Cancel("Error checking database")
		return
	}

	if keyExists && keyMgr.CanLoad() {
		tap.Message("Master key already initialized!")
		tap.Outro("Run 'ply setup' to add a provider.")
		return
	}

	if keyExists && !keyMgr.CanLoad() {
		runRecovery(ctx, keyMgr, db)
		return
	}

	if !keyExists && hasDatabase {
		runRecovery(ctx, keyMgr, db)
		return
	}

	masterKey, err := createMasterKey(ctx, keyMgr)
	if err != nil {
		tap.Cancel("Error creating master key")
		return
	}

	for i := range masterKey {
		masterKey[i] = 0
	}

	tap.Outro("Run 'ply setup' to add a provider.")
}

func runRecovery(ctx context.Context, keyMgr *keys.Manager, db *database.Database) {
	tap.Message("Your configuration could not be decrypted.")

	if err := db.Load(ctx); err != nil {
		tap.Cancel("Error loading database")
		return
	}

	entries := db.ListEntries()
	if len(entries) == 0 {
		tap.Message("No providers configured. Creating fresh master key.")
		masterKey, err := createMasterKey(ctx, keyMgr)
		if err != nil {
			tap.Cancel("Error creating master key")
			return
		}
		for i := range masterKey {
			masterKey[i] = 0
		}
		tap.Outro("Run 'ply setup' to add a provider.")
		return
	}

	providerNames := make([]string, len(entries))
	for i, e := range entries {
		providerNames[i] = e.Provider
	}

	tap.Message(fmt.Sprintf("Provider(s) found: %s", strings.Join(providerNames, ", ")))

	if !prompt.Confirm(ctx, "Re-enter API keys for these providers?") {
		tap.Cancel("Recovery cancelled")
		return
	}

	if err := keyMgr.Delete(); err != nil {
		tap.Cancel("Error removing old configuration")
		return
	}

	masterKey, err := createMasterKey(ctx, keyMgr)
	if err != nil {
		tap.Cancel("Error creating master key")
		return
	}
	defer zeroMasterKey(masterKey)

	for _, entry := range entries {
		apiKey, err := prompt.PromptAPIKey(ctx, entry.Provider)
		if err != nil {
			tap.Cancel(fmt.Sprintf("Error reading API key for %s", entry.Provider))
			return
		}

		cipher, err := crypto.Encrypt(string(masterKey), apiKey)
		if err != nil {
			tap.Cancel(fmt.Sprintf("Error encrypting API key for %s", entry.Provider))
			return
		}

		entry.Cipher = cipher
		entry.UpdatedAt = time.Now().UTC()
		if err := db.UpdateEntry(entry); err != nil {
			tap.Cancel(fmt.Sprintf("Error updating %s", entry.Provider))
			return
		}
	}

	if err := db.Save(ctx); err != nil {
		tap.Cancel("Error saving database")
		return
	}

	tap.Message(fmt.Sprintf("Updated %d provider(s)", len(entries)))
	tap.Outro("Configuration recovered successfully!")
}
