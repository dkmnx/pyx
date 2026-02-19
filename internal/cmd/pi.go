package cmd

import (
	"github.com/dkmnx/ply/internal/pi"
	"github.com/spf13/cobra"
)

var piCmd = &cobra.Command{
	Use:   "pi",
	Short: "Manage pi installation",
	Long:  `Pi command handles installation of the pi coding agent.`,
}

func init() {
	rootCmd.AddCommand(piCmd)

	piCmd.AddCommand(piInstallCmd)
}

var piInstallCmd = &cobra.Command{
	Use:   "install",
	Short: "Install the pi coding agent",
	Long:  `Install installs the pi coding agent using npm.`,
	Run:   runPiInstall,
}

func runPiInstall(cmd *cobra.Command, args []string) {
	cmd.Println("Installing pi coding agent...")

	if err := pi.Install(); err != nil {
		cmd.Printf("Error: %v\n", err)
		return
	}

	cmd.Println("✓ Installation complete")

	if version, err := pi.Version(); err == nil {
		cmd.Printf("pi version: %s\n", version)
	}
}
