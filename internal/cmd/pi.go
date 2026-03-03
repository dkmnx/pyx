package cmd

import (
	"context"

	"github.com/dkmnx/ply/internal/pi"
	"github.com/dkmnx/ply/internal/prompt"
	"github.com/spf13/cobra"
)

var piCmd = &cobra.Command{
	Use:   "pi",
	Short: "Manage pi installation",
	Long:  `Pi command handles installation of the pi coding agent using npm, pnpm, yarn, or bun.`,
}

func init() {
	rootCmd.AddCommand(piCmd)

	piCmd.AddCommand(piInstallCmd)
}

var piInstallCmd = &cobra.Command{
	Use:   "install",
	Short: "Install the pi coding agent",
	Long:  `Install installs the pi coding agent using npm, pnpm, yarn, or bun.`,
	Run:   runPiInstall,
}

var autoDetectPM bool

func init() {
	piInstallCmd.Flags().BoolVar(&autoDetectPM, "auto", false, "Auto-detect package manager without prompting")
}

func runPiInstall(cmd *cobra.Command, args []string) {
	ctx := cmd.Context()
	if ctx == nil {
		ctx = context.Background()
	}

	// Check if pi is already installed
	installed, err := pi.CheckInstalled()
	if err == nil && installed {
		cmd.Println("pi is already installed")
		if version, err := pi.Version(); err == nil {
			cmd.Printf("pi version: %s\n", version)
		}
		return
	}

	cmd.Println("Installing pi coding agent...")

	var pm string
	if autoDetectPM {
		// Auto-detect package manager (empty string triggers auto-detection)
		pm = ""
	} else {
		// Prompt for package manager selection
		pm, err = prompt.PromptPackageManager(ctx)
		if err != nil {
			cmd.Printf("Error selecting package manager: %v\n", err)
			return
		}
	}

	if err := pi.Install(pm); err != nil {
		cmd.Printf("Error: %v\n", err)
		return
	}

	cmd.Println("✓ Installation complete")

	if version, err := pi.Version(); err == nil {
		cmd.Printf("pi version: %s\n", version)
	}
}
