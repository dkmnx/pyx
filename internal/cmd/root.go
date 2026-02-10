package cmd

import (
	"github.com/spf13/cobra"
)

var rootCmd = &cobra.Command{
	Use:   "ply",
	Short: "A CLI tool for managing AI providers",
	Long:  `Ply is a command-line tool for managing and configuring AI provider configurations.`,
}

// Execute runs the root command.
func Execute() error {
	return rootCmd.Execute()
}

func init() {
	rootCmd.CompletionOptions.DisableDefaultCmd = false
}
