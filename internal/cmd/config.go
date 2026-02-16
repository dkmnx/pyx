package cmd

import (
	"github.com/spf13/cobra"
)

var configCmd = &cobra.Command{
	Use:   "config",
	Short: "Manage provider configurations",
	Long:  `Config commands for listing, editing, and deleting provider configurations.`,
}

func init() {
	rootCmd.AddCommand(configCmd)
}
