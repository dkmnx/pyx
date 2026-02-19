package cmd

import (
	"github.com/dkmnx/ply/internal/fs"
	"github.com/dkmnx/ply/internal/keys"
	"github.com/spf13/cobra"
)

var initCmd = &cobra.Command{
	Use:   "init",
	Short: "Initialize ply master key",
	Long:  `Init creates the master encryption key if it doesn't already exist.`,
	Run:   runInit,
}

func init() {
	rootCmd.AddCommand(initCmd)
}

// runInit initializes the ply master key.
//
// The init process creates the data directory if needed, attempts to migrate
// from a legacy master key file, and creates a new master key if one doesn't
// already exist. If a master key already exists, it reports that and exits.
func runInit(cmd *cobra.Command, args []string) {
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
		return
	}
	if migrated {
		cmd.Println("Migrated master key to secure storage.")
	}

	// Check if master key already exists
	keyExists, err := keyMgr.Exists()
	if err != nil {
		cmd.Printf("Error checking master key: %v\n", err)
		return
	}

	if keyExists {
		cmd.Println("Master key already initialized.")
		cmd.Println("Run 'ply setup' to add a provider.")
		return
	}

	// Create new master key
	masterKey, err := createMasterKey(cmd, keyMgr)
	if err != nil {
		cmd.Printf("%v\n", err)
		return
	}

	_ = masterKey // Variable used but not needed after initialization

	cmd.Println()
	cmd.Println("✓ Ply initialized successfully.")
	cmd.Println("Run 'ply setup' to add a provider.")
}
