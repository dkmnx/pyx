package cmd

import (
	"github.com/spf13/cobra"
)

var configListCmd = &cobra.Command{
	Use:    "list",
	Short:  "List all configured providers (deprecated: use 'ply list' instead)",
	Long:   `List displays all providers configured in ply configuration. Deprecated: use 'ply list' which also shows available models.`,
	Hidden: true,
	Run:    runConfigList,
}

func init() {
	configCmd.AddCommand(configListCmd)
}

// runConfigList displays all configured providers in the ply configuration.
// Deprecated: use runList instead which shows models too.
func runConfigList(cmd *cobra.Command, args []string) {
	// Redirect to the new list command
	runList(cmd, args)
}
