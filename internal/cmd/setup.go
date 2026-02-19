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

// getMasterKey retrieves or creates the master key.
// If the key exists, it loads it (prompting for password if needed).
// If the key doesn't exist, it creates and saves a new one.
func getMasterKey(cmd *cobra.Command, keyMgr *keys.Manager) ([]byte, error) {
	keyExists, err := keyMgr.Exists()
	if err != nil {
		return nil, err
	}

	if keyExists {
		return loadExistingMasterKey(cmd, keyMgr)
	}
	return createMasterKey(cmd, keyMgr)
}

// loadExistingMasterKey loads the existing master key.
func loadExistingMasterKey(cmd *cobra.Command, keyMgr *keys.Manager) ([]byte, error) {
	cmd.Printf("✓ Using existing master key\n")

	requiresPassword, err := keyMgr.RequiresPassword()
	if err != nil {
		return nil, fmt.Errorf("error checking password requirement: %w", err)
	}

	var password []byte
	if requiresPassword {
		pwStr, err := prompt.PromptPassword(cmd, "Enter password to unlock your API keys")
		if err != nil {
			return nil, fmt.Errorf("error: %w", err)
		}
		password = []byte(pwStr)
	}

	masterKey, err := keyMgr.Load(password)
	if err != nil {
		if err == keys.ErrInvalidPassword {
			cmd.Println("Password incorrect.")
		}
		return nil, fmt.Errorf("error loading master key: %w", err)
	}

	return masterKey, nil
}

// createMasterKey creates and saves a new master key.
func createMasterKey(cmd *cobra.Command, keyMgr *keys.Manager) ([]byte, error) {
	cmd.Println("Initializing ply for the first time...")

	masterKey, err := keys.GenerateKey()
	if err != nil {
		return nil, fmt.Errorf("error generating master key: %w", err)
	}

	// Try to save to keyring first
	if err := keyMgr.Save(masterKey); err != nil {
		// Keyring unavailable, need password
		cmd.Println("OS keyring unavailable. A password will be used to encrypt your master key.")
		cmd.Println("You will need to enter this password each time you run ply.")

		pwStr, err := prompt.PromptNewPassword(cmd)
		if err != nil {
			return nil, fmt.Errorf("error: %w", err)
		}

		// Set password for encryption
		if err := keyMgr.SetPassword([]byte(pwStr)); err != nil {
			return nil, fmt.Errorf("error saving password: %w", err)
		}

		// Save master key with password encryption
		if err := keyMgr.Save(masterKey); err != nil {
			return nil, fmt.Errorf("error saving master key: %w", err)
		}
	}

	cmd.Printf("✓ Master key initialized\n")
	return masterKey, nil
}

// confirmProviderOverride prompts the user to confirm overriding an existing provider.
func confirmProviderOverride(cmd *cobra.Command, provider string) (bool, error) {
	cmd.Printf("Provider '%s' already configured. Override? (y/N): ", provider)
	confirm, err := prompt.ReadLine()
	if err != nil {
		return false, fmt.Errorf("error reading input: %w", err)
	}
	confirm = strings.TrimSpace(strings.ToLower(confirm))
	if confirm != confirmY && confirm != confirmYes {
		return false, nil
	}
	return true, nil
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

	// Initialize key manager
	keyMgr := keys.New(dataDir)

	// Attempt to migrate from legacy master key file if it exists
	migrated, err := keyMgr.MigrateFromLegacy()
	if err != nil {
		cmd.Printf("Error migrating master key: %v\n", err)
		cmd.Println("Run 'ply setup' to initialize ply.")
		return
	}
	if migrated {
		cmd.Println("Migrated master key to secure storage.")
	}

	// Get master key (load existing or create new)
	masterKey, err := getMasterKey(cmd, keyMgr)
	if err != nil {
		cmd.Printf("%v\n", err)
		return
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
		confirmed, err := confirmProviderOverride(cmd, provider)
		if err != nil {
			cmd.Printf("%v\n", err)
			return
		}
		if !confirmed {
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
