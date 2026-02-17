package cmd

import (
	"context"
	"strings"
	"time"

	"github.com/dkmnx/ply/internal/crypto"
	"github.com/dkmnx/ply/internal/database"
	"github.com/dkmnx/ply/internal/fs"
	"github.com/dkmnx/ply/internal/prompt"
	"github.com/spf13/cobra"
)

var setupCmd = &cobra.Command{
	Use:   "setup",
	Short: "Initialize ply configuration",
	Long:  `Setup creates the necessary configuration directory and files for ply.`,
	Run:   runSetup,
}

func init() {
	rootCmd.AddCommand(setupCmd)
}

// runSetup initializes ply configuration and adds a new provider.
//
// The setup process creates the data directory if needed, initializes or
// loads the master encryption key, prompts the user for provider selection
// and API key, then encrypts and stores the credentials securely.
func runSetup(cmd *cobra.Command, args []string) {
	// Create data directory
	dataDir, err := fs.EnsureDataDir()
	if err != nil {
		cmd.Printf("Error creating data directory: %v\n", err)
		return
	}

	// Load or generate master key
	masterKey, err := fs.LoadMasterKey()
	if err != nil {
		// Generate new master key if doesn't exist
		masterKey, err = crypto.GenerateKey()
		if err != nil {
			cmd.Printf("Error generating master key: %v\n", err)
			return
		}

		// Save master key
		if err := fs.SaveMasterKey(masterKey); err != nil {
			cmd.Printf("Error saving master key: %v\n", err)
			return
		}

		cmd.Printf("✓ Master key initialized\n")
	} else {
		cmd.Printf("✓ Using existing master key\n")
	}

	// Load database
	db := database.New(dataDir)
	if err := db.Load(context.Background()); err != nil {
		cmd.Printf("Error loading database: %v\n", err)
		return
	}

	// Prompt for provider
	provider, err := prompt.PromptProvider(cmd)
	if err != nil {
		cmd.Printf("Error: %v\n", err)
		return
	}

	// Check for existing provider
	if _, err := db.GetEntry(provider); err == nil {
		cmd.Printf("Provider '%s' already configured. Override? (y/N): ", provider)
		confirm, readErr := prompt.ReadLine()
		if readErr != nil {
			cmd.Printf("Error reading input: %v\n", readErr)
			return
		}
		confirm = strings.TrimSpace(strings.ToLower(confirm))
		if confirm != "y" && confirm != confirmYes {
			cmd.Println("Setup cancelled.")
			return
		}
	}

	// Prompt for API key
	apiKey, err := prompt.PromptAPIKey(cmd)
	if err != nil {
		cmd.Printf("Error: %v\n", err)
		return
	}

	// Encrypt API key
	cipher, nonce, err := crypto.Encrypt(masterKey, apiKey)
	if err != nil {
		cmd.Printf("Error encrypting API key: %v\n", err)
		return
	}

	// Create or update entry
	var entry database.Entry
	if existing, err := db.GetEntry(provider); err == nil {
		existing.Cipher = cipher
		existing.Nonce = nonce
		existing.UpdatedAt = time.Now().UTC()
		entry = existing
		if updateErr := db.UpdateEntry(entry); updateErr != nil {
			cmd.Printf("Error updating entry: %v\n", updateErr)
			return
		}
	} else {
		entry = database.NewEntry(provider, cipher, nonce)
		if addErr := db.AddEntry(entry); addErr != nil {
			cmd.Printf("Error adding entry: %v\n", addErr)
			return
		}
	}

	// Save database
	if err := db.Save(context.Background()); err != nil {
		cmd.Printf("Error saving database: %v\n", err)
		return
	}

	// Output confirmation
	cmd.Printf("  ❯ %s\n", entry.Provider)
	cmd.Printf("    Created  : %s\n", entry.CreatedAt.Format(timeFormat))
	cmd.Println()
	cmd.Printf("✓ API key stored securely\n")
}
