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

func runSetup(cmd *cobra.Command, args []string) {
	ctx := context.Background()

	tap.Intro("ply setup")

	dataDir, err := fs.EnsureDataDir()
	if err != nil {
		tap.Cancel(fmt.Sprintf("Error creating data directory: %v", err))
		return
	}

	keyMgr := keys.New(dataDir)
	db := database.New(dataDir)

	_, err = keyMgr.MigrateFromLegacy(db)
	if err != nil {
		tap.Cancel(fmt.Sprintf("Error migrating master key: %v", err))
		tap.Message("Run 'ply init' to initialize ply.")
		return
	}

	masterKey, err := getMasterKey(ctx, keyMgr)
	if err != nil {
		tap.Cancel(fmt.Sprintf("%v", err))
		return
	}

	if err := db.Load(ctx); err != nil {
		tap.Cancel(fmt.Sprintf("Error loading database: %v", err))
		return
	}

	spinner := tap.NewSpinner(tap.SpinnerOptions{})
	spinner.Start("Fetching providers...")

	if err := models.FetchAndCache(ctx); err != nil {
		spinner.Stop("Failed", 1)
		tap.Cancel(fmt.Sprintf("Error fetching providers: %v", err))
		return
	}
	spinner.Stop("Done", 0)

	provider, err := prompt.PromptProvider(ctx)
	if err != nil {
		tap.Cancel(fmt.Sprintf("Error: %v", err))
		return
	}

	if _, err := db.GetEntry(provider); err == nil {
		if !confirmProviderOverride(ctx, provider) {
			tap.Message("Setup cancelled.")
			return
		}
	}

	apiKey, err := prompt.PromptAPIKey(ctx)
	if err != nil {
		tap.Cancel(fmt.Sprintf("Error: %v", err))
		return
	}

	cipher, nonce, err := crypto.Encrypt(masterKey, apiKey)
	if err != nil {
		tap.Cancel(fmt.Sprintf("Error encrypting API key: %v", err))
		return
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
			tap.Cancel(fmt.Sprintf("Error updating entry: %v", updateEntryErr))
			return
		}
	} else {
		entry = database.NewEntry(provider, cipher, nonce)
		if addErr := db.AddEntry(entry); addErr != nil {
			tap.Cancel(fmt.Sprintf("Error adding entry: %v", addErr))
			return
		}
	}

	if err := db.Save(ctx); err != nil {
		tap.Cancel(fmt.Sprintf("Error saving database: %v", err))
		return
	}

	action := "Created"
	if isUpdate {
		action = "Updated"
	}
	tap.Message(fmt.Sprintf("  %s: %s (%s)", entry.Provider, action, entry.UpdatedAt.Format(timeFormat)))

	tap.Outro("API key stored securely")
}
