package cmd

import (
	"github.com/spf13/cobra"
)

var configDeleteCmd = &cobra.Command{
	Use:   "delete [provider name or id]",
	Short: "Delete a provider configuration",
	Long:  `Delete removes a provider configuration by name or ID.`,
	Args:  cobra.ExactArgs(1),
	Run:   runConfigDelete,
}

func init() {
	// Add flags if needed
}

func runConfigDelete(cmd *cobra.Command, args []string) {
	provider := args[0]
	// TODO: Implement delete logic
	cmd.Printf("Deleting provider: %s\n", provider)
}
