package cmd

import (
	"context"
	"time"

	"github.com/dkmnx/ply/internal/crypto"
	"github.com/dkmnx/ply/internal/database"
	"github.com/dkmnx/ply/internal/fs"
	"github.com/dkmnx/ply/internal/prompt"
	"github.com/spf13/cobra"
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
	target := args[0]

	dataDir, err := fs.DataDir()
	if err != nil {
		cmd.Printf("Error getting data directory: %v\n", err)
		return
	}

	db := database.New(dataDir)
	if loadErr := db.Load(context.Background()); loadErr != nil {
		cmd.Printf("Error loading database: %v\n", loadErr)
		return
	}

	entry, err := findEntry(cmd, db, target)
	if err != nil {
		return
	}

	cmd.Printf("Editing provider '%s'\n", entry.Provider)
	cmd.Printf("  Created : %s\n\n", entry.CreatedAt.Format(timeFormat))

	masterKey, err := fs.LoadMasterKey()
	if err != nil {
		cmd.Printf("Error loading master key: %v\n", err)
		return
	}

	newAPIKey, err := prompt.PromptAPIKey(cmd)
	if err != nil {
		cmd.Printf("Error: %v\n", err)
		return
	}

	cipher, nonce, err := crypto.Encrypt(masterKey, newAPIKey)
	if err != nil {
		cmd.Printf("Error encrypting API key: %v\n", err)
		return
	}

	entry.Cipher = cipher
	entry.Nonce = nonce
	entry.UpdatedAt = time.Now().UTC()

	if updateErr := db.UpdateEntry(entry); updateErr != nil {
		cmd.Printf("Error updating entry: %v\n", updateErr)
		return
	}

	if saveErr := db.Save(context.Background()); saveErr != nil {
		cmd.Printf("Error saving database: %v\n", saveErr)
		return
	}

	cmd.Printf("✓ Provider '%s' updated\n", entry.Provider)
}

func findEntry(cmd *cobra.Command, db *database.Database, target string) (database.Entry, error) {
	entry, err := db.GetEntry(target)
	if err != nil {
		cmd.Printf("Error: provider '%s' not found\n", target)
		cmd.Println("Use 'ply config list' to see all configured providers.")
		return database.Entry{}, err
	}
	return entry, nil
}
