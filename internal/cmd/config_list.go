package cmd

import (
	"github.com/dkmnx/ply/internal/database"
	"github.com/dkmnx/ply/internal/fs"
	"github.com/spf13/cobra"
)

//nolint:unused // Used in init() via config.go
var configListCmd = &cobra.Command{
	Use:   "list",
	Short: "List all configured providers",
	Long:  `List displays all providers configured in ply configuration.`,
	Run:   runConfigList,
}

// runConfigList displays all configured providers in the ply configuration.
//
// Reads the database, retrieves all provider entries, and displays them
// in a formatted list. The default provider is marked with "(default)".
// Shows total count of configured providers at the end.
func runConfigList(cmd *cobra.Command, args []string) {
	// Get data directory
	dataDir, err := fs.DataDir()
	if err != nil {
		cmd.Printf("Error getting data directory: %v\n", err)
		return
	}

	// Load database
	db := database.New(dataDir)
	if err := db.Load(); err != nil {
		cmd.Printf("Error loading database: %v\n", err)
		return
	}

	// Load default provider ID
	defaultID, _ := fs.LoadDefaultProvider() //nolint:errcheck // Optional: default may not be set

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
		// Show (default) marker if this is the default
		label := e.Label
		if e.ID == defaultID {
			label = e.Label + " (default)"
		}

		cmd.Printf("  ❯ %s\n", label)
		cmd.Printf("    ID       : %s\n", e.ID)
		cmd.Printf("    Provider : %s\n", e.Provider)
		if e.DefaultModel != "" {
			cmd.Printf("    Model    : %s\n", e.DefaultModel)
		}
		cmd.Printf("    Created  : %s\n", e.CreatedAt.Format(timeFormat))
		cmd.Println()
	}

	cmd.Printf("Total: %d provider(s)\n", len(entries))
}
