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
	Long:  `Delete removes a provider configuration. If no provider is specified, shows an interactive selection.`,
	Args:  cobra.MaximumNArgs(1),
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

	fmt.Println()

	tap.Intro("Delete Provider")

	entries := db.ListEntries()
	if len(entries) == 0 {
		tap.Message("No providers configured.")
		return
	}

	var target string
	if len(args) > 0 {
		target = args[0]
		// Validate provider name before checking database
		if err := providers.Validate(target); err != nil {
			tap.Cancel(fmt.Sprintf("%v", err))
			return
		}
	} else {
		// Interactive selection
		options := make([]tap.SelectOption[string], len(entries))
		for i, entry := range entries {
			options[i] = tap.SelectOption[string]{
				Value: entry.Provider,
				Label: entry.Provider,
			}
		}

		target = tap.Select(ctx, tap.SelectOptions[string]{
			Message: "Select a provider to delete:",
			Options: options,
		})

		if target == "" {
			tap.Cancel("Delete cancelled.")
			return
		}
	}

	entry, err := db.GetEntry(target)
	if err != nil {
		tap.Cancel(fmt.Sprintf("Provider '%s' not found", target))
		tap.Message("Use 'ply list' to see all configured providers.")
		return
	}

	tap.Message(fmt.Sprintf("Provider '%s'", entry.Provider))

	if !prompt.Confirm(ctx, "Are you sure you want to delete this provider?") {
		tap.Cancel("Delete cancelled.")
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
