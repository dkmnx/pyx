package cmd

import (
	"github.com/dkmnx/ply/internal/database"
	"github.com/dkmnx/ply/internal/fs"
	"github.com/spf13/cobra"
)

var defaultCmd = &cobra.Command{
	Use:   "default [label|id]",
	Short: "Set or view the default provider",
	Long:  `Default sets a provider as the default, or displays the current default provider if no argument is provided.`,
	Run:   runDefault,
}

//nolint:unused // Used in init() via config.go
func init() {
	rootCmd.AddCommand(defaultCmd)
}

func runDefault(cmd *cobra.Command, args []string) {
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

	if len(args) == 0 {
		// Display current default provider
		displayDefault(cmd, db)
		return
	}

	// Set default provider
	setDefault(cmd, db, args[0])
}

func displayDefault(cmd *cobra.Command, db *database.Database) {
	defaultID, err := fs.LoadDefaultProvider()
	if err != nil {
		if err == fs.ErrDefaultNotFound {
			cmd.Println("No default provider set.")
			cmd.Println("Use 'ply default [label|id]' to set a default provider.")
		} else {
			cmd.Printf("Error loading default provider: %v\n", err)
		}
		return
	}

	// Look up the default provider
	entry, err := db.GetEntry(defaultID)
	if err != nil {
		cmd.Printf("Error: default provider entry not found (ID: %s)\n", defaultID)
		cmd.Println("The default provider may have been deleted.")
		cmd.Println("Use 'ply default [label|id]' to set a new default provider.")
		return
	}

	// Display default provider
	cmd.Printf("Default provider:\n")
	cmd.Printf("  Label    : %s\n", entry.Label)
	cmd.Printf("  ID       : %s\n", entry.ID)
	cmd.Printf("  Provider : %s\n", entry.Provider)
	cmd.Printf("  Created  : %s\n", entry.CreatedAt.Format("2006-01-02T15:04:05Z"))
}

func setDefault(cmd *cobra.Command, db *database.Database, target string) {
	// Try to find entry by label first
	entry, err := db.GetEntryByLabel(target)
	if err == nil {
		// Found by label
		if err := fs.SaveDefaultProvider(entry.ID); err != nil {
			cmd.Printf("Error saving default provider: %v\n", err)
			return
		}

		cmd.Printf("✓ Default provider set to '%s' (ID: %s)\n", entry.Label, entry.ID)
		return
	}

	// If not found by label, try by ID
	entry, err = db.GetEntry(target)
	if err == nil {
		// Found by ID
		if err := fs.SaveDefaultProvider(entry.ID); err != nil {
			cmd.Printf("Error saving default provider: %v\n", err)
			return
		}

		cmd.Printf("✓ Default provider set to '%s' (ID: %s)\n", entry.Label, entry.ID)
		return
	}

	// Not found by either label or ID
	cmd.Printf("Error: provider '%s' not found\n", target)
	cmd.Println("Use 'ply config list' to see all configured providers.")
}
