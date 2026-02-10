package cmd

import (
	"encoding/json"

	"github.com/dkmnx/ply/internal/database"
	"github.com/dkmnx/ply/internal/fs"
	"github.com/spf13/cobra"
)

type listEntry struct {
	ID        string `json:"id"`
	Label     string `json:"label"`
	Provider  string `json:"provider"`
	CreatedAt string `json:"created_at"`
}

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

	// Convert to output format (exclude cipher and nonce)
	output := make([]listEntry, len(entries))
	for i, e := range entries {
		output[i] = listEntry{
			ID:        e.ID,
			Label:     e.Label,
			Provider:  e.Provider,
			CreatedAt: e.CreatedAt.Format("2006-01-02T15:04:05Z"),
		}
	}

	// Output as pretty JSON
	outputJSON, err := json.MarshalIndent(output, "", "  ")
	if err != nil {
		cmd.Printf("Error formatting output: %v\n", err)
		return
	}

	cmd.Printf("%s\n", outputJSON)
	cmd.Printf("\nTotal: %d provider(s)\n", len(entries))
}
