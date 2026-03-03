package cmd

import (
	"context"

	"github.com/dkmnx/ply/internal/database"
	"github.com/dkmnx/ply/internal/fs"
	"github.com/spf13/cobra"
)

var configListCmd = &cobra.Command{
	Use:   "list",
	Short: "List all configured providers",
	Long:  `List displays all providers configured in ply configuration.`,
	Run:   runConfigList,
}

func init() {
	configCmd.AddCommand(configListCmd)
}

// runConfigList displays all configured providers in the ply configuration.
//
// Reads the database, retrieves all provider entries, and displays them
// in a formatted list. Shows total count of configured providers at the end.
func runConfigList(cmd *cobra.Command, args []string) {
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

	// Output header
	cmd.Println("Configured providers:")
	cmd.Println()

	// Output each entry
	for _, e := range entries {
		cmd.Printf("  ❯ %s\n", e.Provider)
		cmd.Printf("    Created  : %s\n", e.CreatedAt.Format(timeFormat))
		cmd.Println()
	}

	cmd.Printf("Total: %d provider(s)\n", len(entries))
}
