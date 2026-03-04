package cmd

import (
	"context"
	"errors"
	"fmt"
	"strings"
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

func databaseExists(ctx context.Context, db *database.Database) (bool, error) {
	if err := db.Load(ctx); err != nil {
		return false, err
	}
	entries := db.ListEntries()
	return len(entries) > 0, nil
}

func getMasterKey(ctx context.Context, keyMgr *keys.Manager, db *database.Database) ([]byte, error) {
	keyExists, err := keyMgr.Exists()
	if err != nil {
		return nil, err
	}

	if keyExists && !keyMgr.CanLoad() {
		return nil, keys.ErrInvalidPassword
	}

	if !keyExists {
		hasDatabase, err := databaseExists(ctx, db)
		if err != nil {
			return nil, err
		}
		if hasDatabase {
			return nil, keys.ErrInvalidPassword
		}
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
		passwordStr, err := prompt.PromptPassword(ctx, "Enter password to unlock your API keys")
		if err != nil {
			return nil, fmt.Errorf("error: %w", err)
		}
		password = []byte(passwordStr)
	}

	masterKey, err := keyMgr.Load(password)
	if err != nil {
		if errors.Is(err, keys.ErrInvalidPassword) {
			tap.Message("Password incorrect.")
		}
		return nil, fmt.Errorf("error loading master key: %w", err)
	}

	return masterKey, nil
}

func createMasterKey(ctx context.Context, keyMgr *keys.Manager) ([]byte, error) {
	// Check if password already exists in keyring
	hasPassword, err := keyMgr.PasswordExists()
	if err != nil {
		return nil, fmt.Errorf("error checking keyring: %w", err)
	}

	// If no password, prompt user to create one
	if !hasPassword {
		pwStr, err := prompt.PromptNewPassword(ctx)
		if err != nil {
			return nil, fmt.Errorf("error: %w", err)
		}

		password := []byte(pwStr)
		if err := keyMgr.SetPassword(password); err != nil {
			// Zero password before returning
			for i := range password {
				password[i] = 0
			}
			return nil, fmt.Errorf("error saving password: %w", err)
		}
		// Zero password after use
		for i := range password {
			password[i] = 0
		}
	}

	// Verify the password is actually retrievable before generating master key
	// This catches cases where keyring has a stale/empty entry
	_, err = keyMgr.GetStoredPassword()
	if err != nil {
		// Password exists in keyring but can't be retrieved - clear it and prompt again
		_ = keyMgr.DeletePassword()
		tap.Message("Keyring password is invalid. Please create a new password.")
		return createMasterKey(ctx, keyMgr)
	}

	// Now generate and save the master key
	masterKey, err := keys.GenerateKey()
	if err != nil {
		return nil, fmt.Errorf("error generating master key: %w", err)
	}

	if err := keyMgr.Save(masterKey); err != nil {
		return nil, fmt.Errorf("error saving master key: %w", err)
	}

	tap.Message("Master key initialized!")

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

	spinner.Stop("Fetch complete!", 0)
	return nil
}

// storeProviderEntry encrypts and stores a provider entry.
// Returns true if this was an update, false if it was a new entry.
func storeProviderEntry(
	ctx context.Context,
	db *database.Database,
	masterKey []byte,
	provider string,
	apiKey *crypto.SecureString,
) (bool, error) {
	cipher, err := crypto.Encrypt(string(masterKey), string(apiKey.Bytes()))
	if err != nil {
		return false, fmt.Errorf("error encrypting API key: %w", err)
	}

	// Zero the API key after encryption
	apiKey.Zero()

	var entry database.Entry
	isUpdate := false
	if existing, err := db.GetEntry(provider); err == nil {
		isUpdate = true
		existing.Cipher = cipher
		existing.UpdatedAt = time.Now().UTC()
		entry = existing
		if updateEntryErr := db.UpdateEntry(entry); updateEntryErr != nil {
			return false, fmt.Errorf("error updating entry: %w", updateEntryErr)
		}
	} else {
		entry = database.NewEntry(provider, cipher)
		if addErr := db.AddEntry(entry); addErr != nil {
			return false, fmt.Errorf("error adding entry: %w", addErr)
		}
	}

	if err := db.Save(ctx); err != nil {
		return false, fmt.Errorf("error saving database: %w", err)
	}

	return isUpdate, nil
}

// loadOrCreateMasterKey loads existing master key or creates a new one.
func loadOrCreateMasterKey(ctx context.Context, keyMgr *keys.Manager, db *database.Database) ([]byte, error) {
	masterKey, err := getMasterKey(ctx, keyMgr, db)
	if errors.Is(err, keys.ErrInvalidPassword) || errors.Is(err, keys.ErrNoPassword) {
		runRecovery(ctx, keyMgr, db)
		return nil, fmt.Errorf("recovery completed, run 'ply setup' again")
	}
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
			tap.Outro("Provider already configured!")
			return "", "", fmt.Errorf("setup cancelled")
		}
	}

	apiKey, err := prompt.PromptAPIKey(ctx, provider)
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

func runRecovery(ctx context.Context, keyMgr *keys.Manager, db *database.Database) {
	tap.Message("Your configuration could not be decrypted.")

	if err := db.Load(ctx); err != nil {
		tap.Cancel("Error loading database")
		return
	}

	// Delete old master key and password before creating fresh one
	if err := keyMgr.Delete(); err != nil {
		tap.Cancel("Error removing old configuration")
		return
	}

	entries := db.ListEntries()
	if len(entries) == 0 {
		tap.Message("No providers configured. Creating fresh master key.")
		masterKey, err := createMasterKey(ctx, keyMgr)
		if err != nil {
			tap.Cancel(fmt.Sprintf("Error creating master key: %v", err))
			return
		}
		for i := range masterKey {
			masterKey[i] = 0
		}
		tap.Outro("Run 'ply setup' to add a provider.")
		return
	}

	// Create new master key first before re-entering provider API keys
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

	providerNames := make([]string, len(entries))
	for i, e := range entries {
		providerNames[i] = e.Provider
	}

	tap.Message(fmt.Sprintf("Provider(s) found: %s", strings.Join(providerNames, ", ")))

	if !prompt.Confirm(ctx, "Re-enter API keys for these providers?") {
		tap.Cancel("Recovery cancelled")
		return
	}

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

func runSetup(cmd *cobra.Command, args []string) {
	ctx := cmd.Context()
	if ctx == nil {
		ctx = context.Background()
	}

	fmt.Println()

	tap.Intro("Ply Provider Setup")

	// Initialize data directory and key manager
	dataDir, err := fs.EnsureDataDir()
	if err != nil {
		tap.Cancel(fmt.Sprintf("Error creating data directory: %v", err))
		return
	}

	keyMgr := keys.New(dataDir)
	db := database.New(dataDir)

	// Load or create master key
	masterKey, err := loadOrCreateMasterKey(ctx, keyMgr, db)
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

	// Wrap API key in SecureString for secure handling
	secureAPIKey := crypto.NewSecureString(apiKey)

	// Store provider entry (secureAPIKey will be zeroed inside)
	isUpdate, err := storeProviderEntry(ctx, db, masterKey, provider, secureAPIKey)
	if err != nil {
		tap.Cancel(fmt.Sprintf("Error: %v", err))
		return
	}

	// Report success
	reportStoredEntry(provider, isUpdate)

	tap.Outro("Provider setup complete!")
}
