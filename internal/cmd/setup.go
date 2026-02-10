package cmd

import (
	"encoding/json"

	"github.com/dkmnx/ply/internal/crypto"
	"github.com/dkmnx/ply/internal/database"
	"github.com/dkmnx/ply/internal/fs"
	"github.com/dkmnx/ply/internal/prompt"
	"github.com/spf13/cobra"
)

type setupOutput struct {
	ID        string `json:"id"`
	Label     string `json:"label"`
	Provider  string `json:"provider"`
	CreatedAt string `json:"created_at"`
}

var setupCmd = &cobra.Command{
	Use:   "setup",
	Short: "Initialize ply configuration",
	Long:  `Setup creates the necessary configuration directory and files for ply.`,
	Run:   runSetup,
}

func init() {
	rootCmd.AddCommand(setupCmd)
}

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
	if err := db.Load(); err != nil {
		cmd.Printf("Error loading database: %v\n", err)
		return
	}

	// Prompt for provider
	provider, err := prompt.PromptProvider(cmd)
	if err != nil {
		cmd.Printf("Error: %v\n", err)
		return
	}

	// Prompt for label
	label, err := prompt.PromptLabel(cmd, provider)
	if err != nil {
		cmd.Printf("Error: %v\n", err)
		return
	}

	// Check for duplicate label
	_, err = db.GetEntryByLabel(label)
	if err == nil {
		cmd.Printf("Error: label '%s' already exists\n", label)
		return
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

	// Create entry
	entry := database.NewEntry(label, provider, cipher, nonce)

	// Add to database
	if err := db.AddEntry(entry); err != nil {
		cmd.Printf("Error adding entry: %v\n", err)
		return
	}

	// Save database
	if err := db.Save(); err != nil {
		cmd.Printf("Error saving database: %v\n", err)
		return
	}

	// Output confirmation
	output := setupOutput{
		ID:        entry.ID,
		Label:     entry.Label,
		Provider:  entry.Provider,
		CreatedAt: entry.CreatedAt.Format("2006-01-02T15:04:05Z"),
	}

	outputJSON, err := json.MarshalIndent(output, "", "  ")
	if err != nil {
		cmd.Printf("✓ API key stored securely\n")
		return
	}

	cmd.Printf("%s\n", outputJSON)
	cmd.Printf("✓ API key stored securely\n")
}
