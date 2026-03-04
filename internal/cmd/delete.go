package cmd

import (
	"context"
	"fmt"

	"github.com/dkmnx/ply/internal/database"
	"github.com/dkmnx/ply/internal/fs"
	"github.com/dkmnx/ply/internal/prompt"
	"github.com/dkmnx/ply/internal/providers"
	"github.com/spf13/cobra"
	"github.com/yarlson/tap"
)

var deleteCmd = &cobra.Command{
	Use:   "delete [provider]",
	Short: "Delete a provider configuration",
	Long:  `Delete removes a provider configuration by provider name.`,
	Args:  cobra.ExactArgs(1),
	Run:   runDelete,
}

func init() {
	rootCmd.AddCommand(deleteCmd)
}

func runDelete(cmd *cobra.Command, args []string) {
	ctx := cmd.Context()
	if ctx == nil {
		ctx = context.Background()
	}
	target := args[0]

	// Validate provider name before checking database
	if err := providers.Validate(target); err != nil {
		tap.Cancel(fmt.Sprintf("%v", err))
		return
	}

	dataDir, err := fs.DataDir()
	if err != nil {
		tap.Cancel(fmt.Sprintf("Error getting data directory: %v", err))
		return
	}

	db := database.New(dataDir)
	if err := db.Load(ctx); err != nil {
		tap.Cancel(fmt.Sprintf("Error loading database: %v", err))
		return
	}

	entry, err := db.GetEntry(target)
	if err != nil {
		tap.Cancel(fmt.Sprintf("Provider '%s' not found", target))
		tap.Message("Use 'ply list' to see all configured providers.")
		return
	}

	tap.Message(fmt.Sprintf("Provider '%s'", entry.Provider))

	if !prompt.Confirm(ctx, "Are you sure you want to delete this provider?") {
		tap.Message("Delete cancelled.")
		return
	}

	if err := db.DeleteEntry(entry.Provider); err != nil {
		tap.Cancel(fmt.Sprintf("Error deleting entry: %v", err))
		return
	}

	if err := db.Save(ctx); err != nil {
		tap.Cancel(fmt.Sprintf("Error saving database: %v", err))
		return
	}

	tap.Message(fmt.Sprintf("Provider '%s' deleted", entry.Provider))
}