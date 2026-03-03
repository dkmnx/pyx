package cmd

import (
	"context"
	"fmt"

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

	fmt.Println()

	tap.Intro("Ply Initialization")

	dataDir, err := fs.EnsureDataDir()
	if err != nil {
		tap.Cancel("Error creating data directory")
		return
	}

	keyMgr := keys.New(dataDir)

	keyExists, err := keyMgr.Exists()
	if err != nil {
		tap.Cancel("Error checking master key")
		return
	}

	if keyExists {
		tap.Message("Master key already initialized!")
		tap.Outro("Run 'ply setup' to add a provider.")
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

	tap.Outro("Run 'ply setup' to add a provider.")
}
