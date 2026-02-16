package cmd

import (
	"context"
	"strings"

	"github.com/dkmnx/ply/internal/database"
	"github.com/dkmnx/ply/internal/fs"
	"github.com/dkmnx/ply/internal/prompt"
	"github.com/spf13/cobra"
)

//nolint:unused // Used in init() via config.go
var configDeleteCmd = &cobra.Command{
	Use:   "delete [label|id]",
	Short: "Delete a provider configuration",
	Long:  `Delete removes a provider configuration by label or ID.`,
	Args:  cobra.ExactArgs(1),
	Run:   runConfigDelete,
}

// runConfigDelete removes a provider configuration from the database.
//
// The target can be specified by label or ID. Searches the database for
// a matching entry, removes it, and persists the changes. Displays a
// confirmation message upon successful deletion.
func runConfigDelete(cmd *cobra.Command, args []string) {
	target := args[0]

	// Get data directory
	dataDir, err := fs.DataDir()
	if err != nil {
		cmd.Printf("Error getting data directory: %v\n", err)
		return
	}

	// Load database
	db := database.New(dataDir)
	if err := db.Load(context.Background()); err != nil {
		cmd.Printf("Error loading database: %v\n", err)
		return
	}

	// Find entry by label or ID
	entry, err := db.GetEntryByLabelOrID(target)
	if err != nil {
		cmd.Printf("Error: provider '%s' not found\n", target)
		cmd.Println("Use 'ply config list' to see all configured providers.")
		return
	}

	cmd.Printf("Provider '%s' (ID: %s)\n", entry.Label, entry.ID)
	cmd.Printf("  Provider : %s\n", entry.Provider)
	cmd.Print("\nAre you sure you want to delete this provider? (y/N): ")

	confirmation, err := prompt.ReadLine()
	if err != nil {
		cmd.Printf("Error reading input: %v\n", err)
		return
	}

	confirmation = strings.TrimSpace(strings.ToLower(confirmation))
	if confirmation != "y" && confirmation != confirmYes {
		cmd.Println("Delete cancelled.")
		return
	}

	if err := db.DeleteEntry(entry.ID); err != nil {
		cmd.Printf("Error deleting entry: %v\n", err)
		return
	}

	if err := db.Save(context.Background()); err != nil {
		cmd.Printf("Error saving database: %v\n", err)
		return
	}

	cmd.Printf("✓ Provider '%s' deleted\n", entry.Label)
}
