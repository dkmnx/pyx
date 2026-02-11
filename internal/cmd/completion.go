package cmd

import (
	"fmt"
	"os"

	"github.com/spf13/cobra"
)

var completionCmd = &cobra.Command{
	Use:   "completion [bash|zsh|fish|powershell]",
	Short: "Generate shell completion script",
	Long: `To load completions:

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
`,
	DisableFlagsInUseLine: true,
	ValidArgs:             []string{"bash", "zsh", "fish", "powershell"},
	Args:                  cobra.MatchAll(cobra.ExactArgs(1), cobra.OnlyValidArgs),
	Run:                   runCompletion,
}

func init() {
	rootCmd.AddCommand(completionCmd)
}

func runCompletion(cmd *cobra.Command, args []string) {
	switch args[0] {
	case "bash":
		if err := cmd.Root().GenBashCompletion(cmd.OutOrStdout()); err != nil {
			fmt.Fprintf(os.Stderr, "Error generating bash completion: %v\n", err)
			os.Exit(1)
		}
	case "zsh":
		if err := cmd.Root().GenZshCompletion(cmd.OutOrStdout()); err != nil {
			fmt.Fprintf(os.Stderr, "Error generating zsh completion: %v\n", err)
			os.Exit(1)
		}
	case "fish":
		if err := cmd.Root().GenFishCompletion(cmd.OutOrStdout(), true); err != nil {
			fmt.Fprintf(os.Stderr, "Error generating fish completion: %v\n", err)
			os.Exit(1)
		}
	case "powershell":
		if err := cmd.Root().GenPowerShellCompletionWithDesc(cmd.OutOrStdout()); err != nil {
			fmt.Fprintf(os.Stderr, "Error generating powershell completion: %v\n", err)
			os.Exit(1)
		}
	}
}
