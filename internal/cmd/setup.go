package cmd

import (
	"encoding/json"

	"github.com/dkmnx/ply/internal/crypto"
	"github.com/dkmnx/ply/internal/database"
	"github.com/dkmnx/ply/internal/fs"
	"github.com/dkmnx/ply/internal/prompt"
	"github.com/spf13/cobra"
)

// setupOutput represents the structured JSON output returned after
// successful provider configuration.
type setupOutput struct {
	// ID is the unique identifier for the provider entry.
	ID string `json:"id"`
	// Label is the user-friendly name for the provider.
	Label string `json:"label"`
	// Provider is the provider type name (e.g., "anthropic", "openai").
	Provider string `json:"provider"`
	// DefaultModel is the optional default model for this provider.
	DefaultModel string `json:"default_model,omitempty"`
	// CreatedAt is the ISO 8601 timestamp of entry creation.
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

// runSetup initializes ply configuration and adds a new provider.
//
// The setup process creates the data directory if needed, initializes or
// loads the master encryption key, prompts the user for provider selection,
// label, and API key, then encrypts and stores the credentials securely.
// Outputs the configured provider details in JSON format.
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

	// Prompt for default model (optional)
	defaultModel, err := prompt.PromptDefaultModel(cmd, provider)
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
	entry.DefaultModel = defaultModel

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
		ID:           entry.ID,
		Label:        entry.Label,
		Provider:     entry.Provider,
		DefaultModel: entry.DefaultModel,
		CreatedAt:    entry.CreatedAt.Format("2006-01-02T15:04:05Z"),
	}

	outputJSON, err := json.MarshalIndent(output, "", "  ")
	if err != nil {
		cmd.Printf("✓ API key stored securely\n")
		return
	}

	cmd.Printf("%s\n", outputJSON)
	cmd.Printf("✓ API key stored securely\n")
}
