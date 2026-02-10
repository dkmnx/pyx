package cmd

import (
	"github.com/spf13/cobra"
)

var setupCmd = &cobra.Command{
	Use:   "setup",
	Short: "Initialize ply configuration",
	Long:  `Setup creates the necessary configuration directory and files for ply.`,
	Run:   runSetup,
}

func init() {
	rootCmd.AddCommand(setupCmd)
}

func runSetup(cmd *cobra.Command, args []string) {
	// TODO: Implement setup logic
	cmd.Println("Setup command executed")
}
