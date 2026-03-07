package cmd

import (
	"fmt"
	"os"
	"os/exec"
	"path/filepath"

	"github.com/dkmnx/ply/internal/pi"
	"github.com/dkmnx/ply/internal/validation"
	"github.com/spf13/cobra"
)

var installCompletion bool

var completionCmd = &cobra.Command{
	Use:   "completion [bash|zsh|fish|powershell]",
	Short: "Generate or install shell completion script",
	Long: `Generate or install shell completion for ply.

By default (no arguments), detects current shell and installs completion:
  $ ply completion

To generate completion script to stdout:

Bash:
  $ source <(ply completion bash)

  # To load completions for each session, execute once:
  # Linux:
  $ ply completion bash > /etc/bash_completion.d/ply
  # macOS:
  $ ply completion bash > /usr/local/etc/bash_completion.d/ply

Zsh:
  # If shell completion is not already enabled in your environment,
  # you will need to enable it.  You can execute the following once:
  $ echo "autoload -U compinit; compinit" >> ~/.zshrc

  # To load completions for each session, execute once:
  $ ply completion zsh > "${fpath[1]}/_ply"

  # You will need to start a new shell for this setup to take effect.

fish:
  $ ply completion fish | source

  # To load completions for each session, execute once:
  $ ply completion fish > ~/.config/fish/completions/ply.fish

PowerShell:
  PS> ply completion powershell | Out-String | Invoke-Expression

  # To load completions for every new session, run:
  PS> ply completion powershell > ply.ps1
  # and source this file from your PowerShell profile.

Use --install flag to automatically install completion for the specified shell:
  $ ply completion bash --install
  $ ply completion zsh --install
  $ ply completion fish --install
  $ ply completion powershell --install
`,
	DisableFlagsInUseLine: true,
	ValidArgs:             pi.ShellNames,
	Args:                  cobra.MaximumNArgs(1),
	Run:                   runCompletion,
}

func init() {
	completionCmd.Flags().BoolVarP(&installCompletion, "install", "i", false, "Install completion for the specified shell")
	rootCmd.AddCommand(completionCmd)
}

// runCompletion generates and/or installs shell completion scripts.
func runCompletion(cmd *cobra.Command, args []string) {
	// No arguments: detect current shell and install
	if len(args) == 0 {
		shell := pi.DetectCurrentShell()
		fmt.Printf("Detected shell: %s\n", shell)
		if err := installCompletionForShell(string(shell)); err != nil {
			fmt.Fprintf(os.Stderr, "Error installing completion: %v\n", err)
			os.Exit(1)
		}
		return
	}

	shellArg := args[0]

	// Install flag: install for specified shell
	if installCompletion {
		if err := installCompletionForShell(shellArg); err != nil {
			fmt.Fprintf(os.Stderr, "Error installing completion: %v\n", err)
			os.Exit(1)
		}
		return
	}

	// Default: generate completion script to stdout (backward compatible)
	shellType := pi.ShellType(shellArg)
	switch shellType {
	case pi.ShellBash:
		if err := cmd.Root().GenBashCompletion(cmd.OutOrStdout()); err != nil {
			fmt.Fprintf(os.Stderr, "Error generating bash completion: %v\n", err)
			os.Exit(1)
		}
	case pi.ShellZsh:
		if err := cmd.Root().GenZshCompletion(cmd.OutOrStdout()); err != nil {
			fmt.Fprintf(os.Stderr, "Error generating zsh completion: %v\n", err)
			os.Exit(1)
		}
	case pi.ShellFish:
		if err := cmd.Root().GenFishCompletion(cmd.OutOrStdout(), true); err != nil {
			fmt.Fprintf(os.Stderr, "Error generating fish completion: %v\n", err)
			os.Exit(1)
		}
	case pi.ShellPowerShell:
		if err := cmd.Root().GenPowerShellCompletionWithDesc(cmd.OutOrStdout()); err != nil {
			fmt.Fprintf(os.Stderr, "Error generating powershell completion: %v\n", err)
			os.Exit(1)
		}
	default:
		fmt.Fprintf(os.Stderr, "Error: unsupported shell: %s. Valid shells: %v\n", shellArg, pi.ShellNames)
		os.Exit(1)
	}
}

// installCompletionForShell installs completion for a specific shell.
func installCompletionForShell(shell string) error {
	// Validate shell argument for security
	if err := validation.SanitizeShellArg(shell); err != nil {
		return fmt.Errorf("invalid shell argument: %w", err)
	}

	// Generate completion script
	genCmd := exec.Command("ply", "completion", shell)
	output, err := genCmd.CombinedOutput()
	if err != nil {
		return fmt.Errorf("failed to generate completion script: %w", err)
	}

	// Determine installation path based on shell
	path, err := completionInstallPath(shell)
	if err != nil {
		return err
	}

	// Ensure directory exists
	dir := filepath.Dir(path)
	if err := os.MkdirAll(dir, 0700); err != nil {
		return fmt.Errorf("failed to create directory: %w", err)
	}

	// Write completion script (0600 for security-sensitive files)
	if err := os.WriteFile(path, output, 0600); err != nil {
		return fmt.Errorf("failed to write completion script: %w", err)
	}

	fmt.Printf("✓ Completion script installed for %s shell\n", shell)
	fmt.Printf("  Location: %s\n", path)

	// Show activation instructions
	showActivationInstructions(shell, path)

	return nil
}

// completionInstallPath returns the installation path for a shell's completion script.
func completionInstallPath(shell string) (string, error) {
	home, err := os.UserHomeDir()
	if err != nil {
		return "", fmt.Errorf("failed to get home directory: %w", err)
	}

	shellType := pi.ShellType(shell)
	switch shellType {
	case pi.ShellBash:
		return filepath.Join(home, ".bash_completions", "ply.bash"), nil
	case pi.ShellZsh:
		return filepath.Join(home, ".zsh", "completions", "_ply"), nil
	case pi.ShellFish:
		return filepath.Join(home, ".config", "fish", "completions", "ply.fish"), nil
	case pi.ShellPowerShell:
		return filepath.Join(home, "Documents", "PowerShell", "ply.ps1"), nil
	default:
		return "", fmt.Errorf("unsupported shell: %s", shell)
	}
}

// showActivationInstructions prints shell-specific activation instructions.
func showActivationInstructions(shell, path string) {
	shellType := pi.ShellType(shell)
	switch shellType {
	case pi.ShellZsh:
		fmt.Println("  To enable completions, add to your ~/.zshrc:")
		fmt.Printf("    source %s\n", path)
	case pi.ShellFish:
		fmt.Println("  Completions will be loaded automatically on next shell start")
	case pi.ShellPowerShell:
		fmt.Println("  To enable completions, add to your PowerShell profile:")
		fmt.Printf("    . %s\n", path)
	case pi.ShellBash:
		fmt.Println("  To enable completions, add to your ~/.bashrc:")
		fmt.Printf("    source %s\n", path)
	}
}
