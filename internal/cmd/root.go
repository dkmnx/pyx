package cmd

import (
	"fmt"
	"os"
	"os/exec"

	"github.com/dkmnx/ply/internal/crypto"
	"github.com/dkmnx/ply/internal/database"
	"github.com/dkmnx/ply/internal/fs"
	"github.com/spf13/cobra"
)

var rootCmd = &cobra.Command{
	Use:   "ply [provider|id] [args...]",
	Short: "A CLI tool for managing AI providers",
	Long:  `Ply is a command-line tool for managing and configuring AI provider configurations for pi coding agent.`,
	Run:   runRoot,
	Args:  cobra.ArbitraryArgs,
}

// Execute runs the root command.
func Execute() error {
	return rootCmd.Execute()
}

func init() {
	rootCmd.CompletionOptions.DisableDefaultCmd = false
}

// runRoot is the default behavior when running ply without subcommands.
func runRoot(cmd *cobra.Command, args []string) {
	// Get data directory
	dataDir, err := fs.DataDir()
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error getting data directory: %v\n", err)
		os.Exit(1)
	}

	// Load database
	db := database.New(dataDir)
	if err := db.Load(); err != nil {
		fmt.Fprintf(os.Stderr, "Error loading database: %v\n", err)
		os.Exit(1)
	}

	var entry database.Entry

	// Check if a provider name or id is provided
	// Look for -- delimiter to separate provider args from pi args
	var providerArg string
	var piArgs []string
	skipModelsFilter := false

	for i, arg := range args {
		if arg == "--" {
			skipModelsFilter = true
			// Everything before -- could be a provider argument
			if i > 0 {
				providerArg = args[0]
			}
			// Everything after -- goes to pi
			piArgs = args[i+1:]
			break
		}
	}

	if !skipModelsFilter {
		// No -- delimiter, first arg might be provider
		if len(args) > 0 {
			providerArg = args[0]
			piArgs = args[1:]
		}
	}

	if providerArg != "" {
		// Try to find provider by label first
		entry, err = db.GetEntryByLabel(providerArg)
		if err != nil {
			// If not found by label, try by ID
			entry, err = db.GetEntry(providerArg)
			if err != nil {
				fmt.Fprintf(os.Stderr, "Error: provider '%s' not found\n", providerArg)
				fmt.Fprintf(os.Stderr, "Use 'ply config list' to see all configured providers.\n")
				os.Exit(1)
			}
		}
	} else {
		// No provider specified, use default provider
		defaultID, err := fs.LoadDefaultProvider()
		if err != nil {
			if err == fs.ErrDefaultNotFound {
				fmt.Fprintf(os.Stderr, "No default provider set.\n")
				fmt.Fprintf(os.Stderr, "Run 'ply default [label|id]' to set a default provider.\n")
				fmt.Fprintf(os.Stderr, "Or use 'ply [label|id]' to specify a provider.\n")
				fmt.Fprintf(os.Stderr, "Use 'ply config list' to see all configured providers.\n")
			} else {
				fmt.Fprintf(os.Stderr, "Error loading default provider: %v\n", err)
			}
			os.Exit(1)
		}

		// Get the default provider entry
		entry, err = db.GetEntry(defaultID)
		if err != nil {
			fmt.Fprintf(os.Stderr, "Error: default provider entry not found (ID: %s)\n", defaultID)
			fmt.Fprintf(os.Stderr, "The default provider may have been deleted.\n")
			fmt.Fprintf(os.Stderr, "Use 'ply default [label|id]' to set a new default provider.\n")
			os.Exit(1)
		}
	}

	// Load master key
	masterKey, err := fs.LoadMasterKey()
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error loading master key: %v\n", err)
		fmt.Fprintf(os.Stderr, "Run 'ply setup' to initialize ply.\n")
		os.Exit(1)
	}

	// Decrypt the API key
	apiKey, err := crypto.Decrypt(masterKey, entry.Cipher, entry.Nonce)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error decrypting API key: %v\n", err)
		os.Exit(1)
	}

	// Get the environment variable name for this provider
	envVar, ok := providerEnvVar(entry.Provider)
	if !ok {
		fmt.Fprintf(os.Stderr, "Error: unsupported provider '%s'\n", entry.Provider)
		os.Exit(1)
	}

	// Set the environment variable
	if err := os.Setenv(envVar, apiKey); err != nil {
		fmt.Fprintf(os.Stderr, "Error setting environment variable: %v\n", err)
		os.Exit(1)
	}

	// Build pi arguments
	var finalPiArgs []string
	if !skipModelsFilter {
		// Add models filter only if -- delimiter is not present
		finalPiArgs = []string{"--models", fmt.Sprintf("%s/*", entry.Provider)}
	}
	finalPiArgs = append(finalPiArgs, piArgs...)

	// Run pi with arguments passed through
	piCmd := exec.Command("pi", finalPiArgs...)
	piCmd.Stdin = os.Stdin
	piCmd.Stdout = os.Stdout
	piCmd.Stderr = os.Stderr

	if err := piCmd.Run(); err != nil {
		// If it's an exit error, exit with the same code
		if exitErr, ok := err.(*exec.ExitError); ok {
			os.Exit(exitErr.ExitCode())
		}
		fmt.Fprintf(os.Stderr, "Error running pi: %v\n", err)
		os.Exit(1)
	}
}

// providerEnvVar returns the environment variable name for a given provider.
func providerEnvVar(provider string) (string, bool) {
	envMap := map[string]string{
		"anthropic":              "ANTHROPIC_API_KEY",
		"azure-openai-responses": "AZURE_OPENAI_API_KEY",
		"openai":                 "OPENAI_API_KEY",
		"google":                 "GEMINI_API_KEY",
		"gemini":                 "GEMINI_API_KEY",
		"mistral":                "MISTRAL_API_KEY",
		"groq":                   "GROQ_API_KEY",
		"cerebras":               "CEREBRAS_API_KEY",
		"xai":                    "XAI_API_KEY",
		"openrouter":             "OPENROUTER_API_KEY",
		"vercel-ai-gateway":      "AI_GATEWAY_API_KEY",
		"zai":                    "ZAI_API_KEY",
		"opencode":               "OPENCODE_API_KEY",
		"opencode-zen":           "OPENCODE_API_KEY",
		"huggingface":            "HF_TOKEN",
		"kimi-coding":            "KIMI_API_KEY",
		"minimax":                "MINIMAX_API_KEY",
		"minimax-cn":             "MINIMAX_CN_API_KEY",
	}
	envVar, ok := envMap[provider]
	return envVar, ok
}
