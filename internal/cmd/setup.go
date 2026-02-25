package cmd

import (
	"context"
	"fmt"
	"time"

	"github.com/dkmnx/ply/internal/crypto"
	"github.com/dkmnx/ply/internal/database"
	"github.com/dkmnx/ply/internal/fs"
	"github.com/dkmnx/ply/internal/keys"
	"github.com/dkmnx/ply/internal/models"
	"github.com/dkmnx/ply/internal/prompt"
	"github.com/spf13/cobra"
	"github.com/yarlson/tap"
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

func getMasterKey(ctx context.Context, keyMgr *keys.Manager) ([]byte, error) {
	keyExists, err := keyMgr.Exists()
	if err != nil {
		return nil, err
	}

	if keyExists {
		return loadExistingMasterKey(ctx, keyMgr)
	}
	return createMasterKey(ctx, keyMgr)
}

func loadExistingMasterKey(ctx context.Context, keyMgr *keys.Manager) ([]byte, error) {
	tap.Message("Using existing master key")

	requiresPassword, err := keyMgr.RequiresPassword()
	if err != nil {
		return nil, fmt.Errorf("error checking password requirement: %w", err)
	}

	var password []byte
	if requiresPassword {
		pwStr, err := prompt.PromptPassword(ctx, "Enter password to unlock your API keys")
		if err != nil {
			return nil, fmt.Errorf("error: %w", err)
		}
		password = []byte(pwStr)
	}

	masterKey, err := keyMgr.Load(password)
	if err != nil {
		if err == keys.ErrInvalidPassword {
			tap.Message("Password incorrect.")
		}
		return nil, fmt.Errorf("error loading master key: %w", err)
	}

	return masterKey, nil
}

func createMasterKey(ctx context.Context, keyMgr *keys.Manager) ([]byte, error) {
	tap.Message("Initializing ply for the first time...")

	masterKey, err := keys.GenerateKey()
	if err != nil {
		return nil, fmt.Errorf("error generating master key: %w", err)
	}

	if err := keyMgr.Save(masterKey); err != nil {
		tap.Message("OS keyring unavailable. A password will be used to encrypt your master key.")
		tap.Message("You will need to enter this password each time you run ply.")

		pwStr, err := prompt.PromptNewPassword(ctx)
		if err != nil {
			return nil, fmt.Errorf("error: %w", err)
		}

		password := []byte(pwStr)
		if err := keyMgr.SetPassword(password); err != nil {
			return nil, fmt.Errorf("error saving password: %w", err)
		}
		for i := range password {
			password[i] = 0
		}

		if err := keyMgr.Save(masterKey); err != nil {
			return nil, fmt.Errorf("error saving master key: %w", err)
		}
	}

	tap.Message("Master key initialized")
	return masterKey, nil
}

func confirmProviderOverride(ctx context.Context, provider string) bool {
	return prompt.Confirm(ctx, fmt.Sprintf("Provider '%s' already configured. Override?", provider))
}

// fetchProviders updates the provider model cache with user feedback.
func fetchProviders(ctx context.Context) error {
	spinner := tap.NewSpinner(tap.SpinnerOptions{})
	spinner.Start("Fetching providers...")

	if err := models.FetchAndCache(ctx); err != nil {
		spinner.Stop("Failed", 1)
		return fmt.Errorf("error fetching providers: %w", err)
	}

	spinner.Stop("Done", 0)
	return nil
}

// storeProviderEntry encrypts and stores a provider entry.
// Returns true if this was an update, false if it was a new entry.
func storeProviderEntry(
	ctx context.Context,
	db *database.Database,
	masterKey []byte,
	provider string,
	apiKey string,
) (bool, error) {
	cipher, nonce, err := crypto.Encrypt(masterKey, apiKey)
	if err != nil {
		return false, fmt.Errorf("error encrypting API key: %w", err)
	}

	var entry database.Entry
	isUpdate := false
	if existing, err := db.GetEntry(provider); err == nil {
		isUpdate = true
		existing.Cipher = cipher
		existing.Nonce = nonce
		existing.UpdatedAt = time.Now().UTC()
		entry = existing
		if updateEntryErr := db.UpdateEntry(entry); updateEntryErr != nil {
			return false, fmt.Errorf("error updating entry: %w", updateEntryErr)
		}
	} else {
		entry = database.NewEntry(provider, cipher, nonce)
		if addErr := db.AddEntry(entry); addErr != nil {
			return false, fmt.Errorf("error adding entry: %w", addErr)
		}
	}

	if err := db.Save(ctx); err != nil {
		return false, fmt.Errorf("error saving database: %w", err)
	}

	return isUpdate, nil
}

// handleLegacyMigration handles legacy key migration if needed.
func handleLegacyMigration(ctx context.Context, keyMgr *keys.Manager, db *database.Database) error {
	_, err := keyMgr.MigrateFromLegacy(db)
	if err != nil {
		tap.Cancel(fmt.Sprintf("Error migrating master key: %v", err))
		tap.Message("Run 'ply init' to initialize ply.")
		return err
	}
	return nil
}

// loadOrCreateMasterKey loads existing master key or creates a new one.
func loadOrCreateMasterKey(ctx context.Context, keyMgr *keys.Manager) ([]byte, error) {
	masterKey, err := getMasterKey(ctx, keyMgr)
	if err != nil {
		tap.Cancel(fmt.Sprintf("%v", err))
		return nil, err
	}
	return masterKey, nil
}

// promptProviderAndKey prompts user for provider and API key, handling overrides.
func promptProviderAndKey(ctx context.Context, db *database.Database) (string, string, error) {
	provider, err := prompt.PromptProvider(ctx)
	if err != nil {
		return "", "", fmt.Errorf("error: %w", err)
	}

	// Check if provider already configured
	if _, err := db.GetEntry(provider); err == nil {
		if !confirmProviderOverride(ctx, provider) {
			return "", "", fmt.Errorf("setup cancelled")
		}
	}

	apiKey, err := prompt.PromptAPIKey(ctx)
	if err != nil {
		return "", "", fmt.Errorf("error: %w", err)
	}

	return provider, apiKey, nil
}

// reportStoredEntry prints success message for stored provider.
func reportStoredEntry(provider string, isUpdate bool) {
	action := "Created"
	if isUpdate {
		action = "Updated"
	}
	tap.Message(fmt.Sprintf("  %s: %s (%s)", provider, action, time.Now().UTC().Format(timeFormat)))
}

func runSetup(cmd *cobra.Command, args []string) {
	ctx := context.Background()

	tap.Intro("ply setup")

	// Initialize data directory and key manager
	dataDir, err := fs.EnsureDataDir()
	if err != nil {
		tap.Cancel(fmt.Sprintf("Error creating data directory: %v", err))
		return
	}

	keyMgr := keys.New(dataDir)
	db := database.New(dataDir)

	// Handle legacy migration
	if err := handleLegacyMigration(ctx, keyMgr, db); err != nil {
		return
	}

	// Load or create master key
	masterKey, err := loadOrCreateMasterKey(ctx, keyMgr)
	if err != nil {
		return
	}
	defer zeroMasterKey(masterKey)

	// Load database
	if err := db.Load(ctx); err != nil {
		tap.Cancel(fmt.Sprintf("Error loading database: %v", err))
		return
	}

	// Fetch and cache provider models
	if err := fetchProviders(ctx); err != nil {
		return
	}

	// Prompt for provider and API key
	provider, apiKey, err := promptProviderAndKey(ctx, db)
	if err != nil {
		return
	}

	// Store provider entry
	isUpdate, err := storeProviderEntry(ctx, db, masterKey, provider, apiKey)
	if err != nil {
		tap.Cancel(fmt.Sprintf("Error: %v", err))
		return
	}

	// Report success
	reportStoredEntry(provider, isUpdate)
	tap.Outro("API key stored securely")
}
