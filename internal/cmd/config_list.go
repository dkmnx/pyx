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
	Long:  `List displays all providers configured in the ply configuration.`,
	Run:   runConfigList,
}

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
		cmd.Printf("  ❯ %s\n", e.Label)
		cmd.Printf("    ID       : %s\n", e.ID)
		cmd.Printf("    Provider : %s\n", e.Provider)
		cmd.Printf("    Created  : %s\n", e.CreatedAt.Format("2006-01-02T15:04:05Z"))
		cmd.Println()
	}

	cmd.Printf("Total: %d provider(s)\n", len(entries))
}
