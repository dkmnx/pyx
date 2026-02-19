package cmd

import (
	"fmt"
	"os"

	"github.com/dkmnx/ply/internal/fs"
	"github.com/dkmnx/ply/internal/keys"
	"github.com/spf13/cobra"
)

var resetCmd = &cobra.Command{
	Use:   "reset",
	Short: "Reset ply configuration",
	Long:  `Reset removes all encrypted data and keys from ply.

This is useful when the encryption key is lost or corrupted.
You will need to re-add your providers after running this command.`,
	Run:   runReset,
}

func init() {
	rootCmd.AddCommand(resetCmd)
}

func runReset(cmd *cobra.Command, args []string) {
	fmt.Println("This will delete:")
	fmt.Println("  - All encrypted API keys")
	fmt.Println("  - Master key from OS keyring")
	fmt.Println("  - Master key file (if exists)")
	fmt.Println()
	fmt.Println("⚠️  This action cannot be undone!")
	fmt.Println()

	// Prompt for confirmation
	fmt.Print("Are you sure you want to reset? Type 'yes' to confirm: ")
	var confirm string
	fmt.Scanln(&confirm)

	if confirm != "yes" {
		fmt.Println("Reset cancelled.")
		return
	}

	// Get data directory
	dataDir, err := fs.DataDir()
	if err != nil {
		cmd.Printf("Error getting data directory: %v\n", err)
		return
	}

	// Remove database
	dbPath := dataDir + "/database.json"
	if _, err := os.Stat(dbPath); err == nil {
		if err := os.Remove(dbPath); err != nil {
			cmd.Printf("Error removing database: %v\n", err)
			return
		}
		fmt.Println("✓ Database deleted")
	}

	// Remove database backup
	dbBackupPath := dataDir + "/database.json.bak"
	if _, err := os.Stat(dbBackupPath); err == nil {
		if err := os.Remove(dbBackupPath); err != nil {
			cmd.Printf("Error removing database backup: %v\n", err)
			return
		}
		fmt.Println("✓ Database backup deleted")
	}

	// Remove master key
	keyMgr := keys.New(dataDir)
	if err := keyMgr.Delete(); err != nil {
		cmd.Printf("Error removing master key: %v\n", err)
		return
	}
	fmt.Println("✓ Master key deleted")

	// Remove legacy key file
	legacyKeyPath := dataDir + "/master.key"
	if _, err := os.Stat(legacyKeyPath); err == nil {
		if err := os.Remove(legacyKeyPath); err != nil {
			cmd.Printf("Error removing legacy key file: %v\n", err)
			return
		}
		fmt.Println("✓ Legacy key file deleted")
	}

	// Remove password file
	passwordFile := dataDir + "/password.bin"
	if _, err := os.Stat(passwordFile); err == nil {
		if err := os.Remove(passwordFile); err != nil {
			cmd.Printf("Error removing password file: %v\n", err)
			return
		}
		fmt.Println("✓ Password file deleted")
	}

	fmt.Println()
	fmt.Println("✓ Reset complete!")
	fmt.Println()
	fmt.Println("Next steps:")
	fmt.Println("  1. Run: ply init")
	fmt.Println("  2. Run: ply setup")
	fmt.Println("  3. Re-add your providers")
}
