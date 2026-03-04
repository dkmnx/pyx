package cmd

import (
	"context"
	"fmt"
	"time"

	"github.com/dkmnx/ply/internal/crypto"
	"github.com/dkmnx/ply/internal/database"
	"github.com/dkmnx/ply/internal/fs"
	"github.com/dkmnx/ply/internal/keys"
	"github.com/dkmnx/ply/internal/prompt"
	"github.com/dkmnx/ply/internal/providers"
	"github.com/spf13/cobra"
	"github.com/yarlson/tap"
)

var configEditCmd = &cobra.Command{
	Use:   "edit [provider]",
	Short: "Edit a provider configuration",
	Long:  `Edit allows you to update an existing provider's API key.`,
	Args:  cobra.ExactArgs(1),
	Run:   runConfigEdit,
}

func init() {
	configCmd.AddCommand(configEditCmd)
}

func runConfigEdit(cmd *cobra.Command, args []string) {
	ctx := cmd.Context()
	if ctx == nil {
		ctx = context.Background()
	}
	target := args[0]

	dataDir, err := fs.DataDir()
	if err != nil {
		tap.Cancel(fmt.Sprintf("Error getting data directory: %v", err))
		return
	}

	db := database.New(dataDir)
	if loadErr := db.Load(ctx); loadErr != nil {
		tap.Cancel(fmt.Sprintf("Error loading database: %v", loadErr))
		return
	}

	entry, err := findEntry(db, target)
	if err != nil {
		return
	}

	tap.Message(fmt.Sprintf("Editing provider '%s'", entry.Provider))
	tap.Message(fmt.Sprintf("  Created: %s", entry.CreatedAt.Format(timeFormat)))

	if !prompt.Confirm(ctx, "Update API key for this provider?") {
		tap.Message("Edit cancelled.")
		return
	}

	keyMgr := keys.New(dataDir)
	masterKey, err := loadExistingMasterKey(ctx, keyMgr)
	if err != nil {
		tap.Cancel(fmt.Sprintf("%v", err))
		return
	}

	newAPIKey, err := prompt.PromptAPIKey(ctx, entry.Provider)
	if err != nil {
		tap.Cancel(fmt.Sprintf("Error: %v", err))
		return
	}

	cipher, err := crypto.Encrypt(string(masterKey), newAPIKey)
	if err != nil {
		tap.Cancel(fmt.Sprintf("Error encrypting API key: %v", err))
		return
	}

	entry.Cipher = cipher
	entry.UpdatedAt = time.Now().UTC()

	if updateErr := db.UpdateEntry(entry); updateErr != nil {
		tap.Cancel(fmt.Sprintf("Error updating entry: %v", updateErr))
		return
	}

	if saveErr := db.Save(ctx); saveErr != nil {
		tap.Cancel(fmt.Sprintf("Error saving database: %v", saveErr))
		return
	}

	tap.Message(fmt.Sprintf("Provider '%s' updated", entry.Provider))
}

func findEntry(db *database.Database, target string) (database.Entry, error) {
	// Validate provider name before checking database
	if err := providers.Validate(target); err != nil {
		return database.Entry{}, err
	}

	entry, err := db.GetEntry(target)
	if err != nil {
		tap.Cancel(fmt.Sprintf("Provider '%s' not found", target))
		tap.Message("Use 'ply list' to see all configured providers.")
		return database.Entry{}, err
	}
	return entry, nil
}
