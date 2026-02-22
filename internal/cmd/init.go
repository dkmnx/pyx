package cmd

import (
	"context"

	"github.com/dkmnx/ply/internal/database"
	"github.com/dkmnx/ply/internal/fs"
	"github.com/dkmnx/ply/internal/keys"
	"github.com/spf13/cobra"
	"github.com/yarlson/tap"
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

func runInit(cmd *cobra.Command, args []string) {
	ctx := context.Background()

	tap.Intro("ply init")

	dataDir, err := fs.EnsureDataDir()
	if err != nil {
		tap.Cancel("Error creating data directory")
		return
	}

	keyMgr := keys.New(dataDir)
	db := database.New(dataDir)

	_, err = keyMgr.MigrateFromLegacy(db)
	if err != nil {
		tap.Cancel("Error migrating master key")
		return
	}

	keyExists, err := keyMgr.Exists()
	if err != nil {
		tap.Cancel("Error checking master key")
		return
	}

	if keyExists {
		tap.Message("Master key already initialized.")
		tap.Message("Run 'ply setup' to add a provider.")
		return
	}

	masterKey, err := createMasterKey(ctx, keyMgr)
	if err != nil {
		tap.Cancel("Error creating master key")
		return
	}

	for i := range masterKey {
		masterKey[i] = 0
	}

	tap.Outro("Ply initialized successfully")
	tap.Message("Run 'ply setup' to add a provider.")
}
