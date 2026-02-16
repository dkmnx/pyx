package cmd

import (
	"context"
	"fmt"
	"strings"

	"github.com/dkmnx/ply/internal/crypto"
	"github.com/dkmnx/ply/internal/database"
	"github.com/dkmnx/ply/internal/fs"
	"github.com/dkmnx/ply/internal/models"
	"github.com/dkmnx/ply/internal/prompt"
	"github.com/spf13/cobra"
)

//nolint:unused // Used in init() via config.go
var configEditCmd = &cobra.Command{
	Use:   "edit [provider name or id]",
	Short: "Edit a provider configuration",
	Long:  `Edit allows you to modify an existing provider configuration by name or ID.`,
	Args:  cobra.ExactArgs(1),
	Run:   runConfigEdit,
}

func init() {
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

	cmd.Printf("Editing provider '%s' (ID: %s)\n", entry.Label, entry.ID)
	cmd.Printf("  Provider      : %s\n", entry.Provider)
	if entry.DefaultModel != "" {
		cmd.Printf("  Default Model : %s\n", entry.DefaultModel)
	}
	cmd.Printf("  Created       : %s\n\n", entry.CreatedAt.Format(timeFormat))

	newLabel := promptNewLabel(cmd, entry.Label)
	if newLabel == "" {
		return
	}

	newProvider := promptNewProvider(cmd, entry.Provider)
	if newProvider == "" {
		return
	}

	newDefaultModel := promptNewDefaultModel(cmd, entry.DefaultModel, newProvider)

	updateAPIKey := promptUpdateAPIKey(cmd)
	if !updateAPIKey {
		handleMetadataUpdate(cmd, db, &entry, newLabel, newProvider, newDefaultModel)
		return
	}

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

	entry.Label = newLabel
	entry.Provider = newProvider
	entry.DefaultModel = newDefaultModel
	entry.Cipher = cipher
	entry.Nonce = nonce

	if updateErr := db.UpdateEntry(entry); updateErr != nil {
		if updateErr == database.ErrDuplicateLabel {
			cmd.Printf("Error: label '%s' already exists\n", newLabel)
			return
		}
		cmd.Printf("Error updating entry: %v\n", updateErr)
		return
	}

	if saveErr := db.Save(context.Background()); saveErr != nil {
		cmd.Printf("Error saving database: %v\n", saveErr)
		return
	}

	cmd.Printf("✓ Provider '%s' updated\n", newLabel)
}

func findEntry(cmd *cobra.Command, db *database.Database, target string) (database.Entry, error) {
	entry, err := db.GetEntryByLabel(target)
	if err != nil {
		entry, err = db.GetEntry(target)
		if err != nil {
			cmd.Printf("Error: provider '%s' not found\n", target)
			cmd.Println("Use 'ply config list' to see all configured providers.")
			return database.Entry{}, err
		}
	}
	return entry, nil
}

func handleMetadataUpdate(cmd *cobra.Command, db *database.Database, entry *database.Entry, newLabel, newProvider, newDefaultModel string) {
	if newLabel != entry.Label || newProvider != entry.Provider || newDefaultModel != entry.DefaultModel {
		entry.Label = newLabel
		entry.Provider = newProvider
		entry.DefaultModel = newDefaultModel
		if updateErr := db.UpdateEntry(*entry); updateErr != nil {
			if updateErr == database.ErrDuplicateLabel {
				cmd.Printf("Error: label '%s' already exists\n", newLabel)
				return
			}
			cmd.Printf("Error updating entry: %v\n", updateErr)
			return
		}
		if saveErr := db.Save(context.Background()); saveErr != nil {
			cmd.Printf("Error saving database: %v\n", saveErr)
			return
		}
		cmd.Printf("✓ Provider '%s' updated\n", newLabel)
	}
}

func promptNewLabel(cmd *cobra.Command, currentLabel string) string {
	cmd.Printf("Enter new label (current: %s, press Enter to keep): ", currentLabel)
	input, err := prompt.ReadLine()
	if err != nil {
		cmd.Printf("Error reading input: %v\n", err)
		return ""
	}
	input = strings.TrimSpace(input)
	if input == "" {
		return currentLabel
	}
	return input
}

func promptNewProvider(cmd *cobra.Command, currentProvider string) string {
	cmd.Printf("Current provider: %s\n", currentProvider)
	cmd.Print("Do you want to change the provider? (y/N): ")
	input, err := prompt.ReadLine()
	if err != nil {
		cmd.Printf("Error reading input: %v\n", err)
		return ""
	}
	input = strings.TrimSpace(strings.ToLower(input))
	if input != "y" && input != confirmYes {
		return currentProvider
	}

	provider, err := prompt.PromptProvider(cmd)
	if err != nil {
		cmd.Printf("Error: %v\n", err)
		return ""
	}
	return provider
}

func promptUpdateAPIKey(cmd *cobra.Command) bool {
	cmd.Print("Do you want to update the API key? (y/N): ")
	input, err := prompt.ReadLine()
	if err != nil {
		cmd.Printf("Error reading input: %v\n", err)
		return false
	}
	input = strings.TrimSpace(strings.ToLower(input))
	return input == "y" || input == confirmYes
}

func promptNewDefaultModel(cmd *cobra.Command, currentModel, provider string) string {
	availableModels := models.ForProvider(provider)

	currentDisplay := currentModel
	if currentDisplay == "" {
		currentDisplay = "(none)"
	}

	if len(availableModels) > 0 {
		cmd.Printf("Available models for %s:\n", provider)
		for i, m := range availableModels {
			cmd.Printf("  %d. %s\n", i+1, m)
		}
		cmd.Println()
	}

	cmd.Printf("Enter new default model for %s (current: %s, number or name, press Enter to keep): ", provider, currentDisplay)

	for {
		input, err := prompt.ReadLine()
		if err != nil {
			cmd.Printf("Error reading input: %v\n", err)
			return currentModel
		}
		input = strings.TrimSpace(input)
		if input == "" {
			return currentModel
		}

		// Check if input is a number
		var num int
		if _, err := fmt.Sscanf(input, "%d", &num); err == nil {
			if num >= 1 && num <= len(availableModels) {
				return availableModels[num-1]
			}
			cmd.Printf("Invalid number. Please enter 1-%d or type a model name: ", len(availableModels))
			continue
		}

		// Check if input matches a model name
		for _, m := range availableModels {
			if strings.EqualFold(input, m) {
				return m
			}
		}

		// Allow custom model name if not in list
		return input
	}
}
