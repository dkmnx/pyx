package cmd

import (
	"github.com/spf13/cobra"
)

var defaultCmd = &cobra.Command{
	Use:   "default [provider name or id]",
	Short: "Set or view the default provider",
	Long:  `Default sets a provider as the default, or displays the current default provider if no argument is provided.`,
	Run:   runDefault,
}

func init() {
	rootCmd.AddCommand(defaultCmd)
}

func runDefault(cmd *cobra.Command, args []string) {
	if len(args) == 0 {
		// TODO: Display current default provider
		cmd.Println("Current default provider: [TODO]")
		return
	}
	provider := args[0]
	// TODO: Set default provider logic
	cmd.Printf("Setting default provider to: %s\n", provider)
}
