package cmd

import (
	"github.com/spf13/cobra"
)

var configListCmd = &cobra.Command{
	Use:   "list",
	Short: "List all configured providers",
	Long:  `List displays all providers configured in the ply configuration.`,
	Run:   runConfigList,
}

func init() {
	// Add flags if needed
}

func runConfigList(cmd *cobra.Command, args []string) {
	// TODO: Implement list logic
	cmd.Println("Listing providers...")
}
