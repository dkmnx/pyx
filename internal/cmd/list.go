package cmd

import (
	"context"

	"github.com/dkmnx/ply/internal/database"
	"github.com/dkmnx/ply/internal/fs"
	"github.com/dkmnx/ply/internal/models"
	"github.com/spf13/cobra"
)

var listCmd = &cobra.Command{
	Use:   "list",
	Short: "List configured providers with their models",
	Long:  `List displays all configured providers and their available models.`,
	Run:   runList,
}

func init() {
	rootCmd.AddCommand(listCmd)
}

// runList displays all configured providers with their available models.
//
// Reads the database, retrieves all provider entries, and displays them
// in a formatted list showing the provider name, creation date, and
// available models for each provider. Shows total count at the end.
func runList(cmd *cobra.Command, args []string) {
	ctx := cmd.Context()
	if ctx == nil {
		ctx = context.Background()
	}

	// Get data directory
	dataDir, err := fs.DataDir()
	if err != nil {
		cmd.Printf("Error getting data directory: %v\n", err)
		return
	}

	// Load database
	db := database.New(dataDir)
	if err := db.Load(ctx); err != nil {
		cmd.Printf("Error loading database: %v\n", err)
		return
	}

	// Get all entries
	entries := db.ListEntries()

	if len(entries) == 0 {
		cmd.Println("No providers configured.")
		return
	}

	// Load models from cache
	modelsData, err := models.GetModels(ctx)
	if err != nil {
		cmd.Printf("Warning: could not load models: %v\n", err)
		modelsData = make(models.Models)
	}

	// Output header
	cmd.Println("Configured providers:")
	cmd.Println()

	// Output each entry with models
	for _, e := range entries {
		cmd.Printf("  ❯ %s\n", e.Provider)
		cmd.Printf("    Created  : %s\n", e.CreatedAt.Format(timeFormat))

		// Show models for this provider
		if modelList, ok := modelsData[e.Provider]; ok && len(modelList) > 0 {
			cmd.Printf("    Models   : %d available\n", len(modelList))
			for _, model := range modelList {
				cmd.Printf("      - %s\n", model)
			}
		} else {
			cmd.Println("    Models   : No models found (run 'ply models update' to fetch)")
		}
		cmd.Println()
	}

	cmd.Printf("Total: %d provider(s)\n", len(entries))
}
