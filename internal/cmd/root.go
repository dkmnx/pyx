package cmd

import (
	"context"
	"fmt"
	"os"
	"os/exec"

	"github.com/dkmnx/ply/internal/crypto"
	"github.com/dkmnx/ply/internal/database"
	"github.com/dkmnx/ply/internal/fs"
	"github.com/spf13/cobra"
)

const (
	timeFormat = "2006-01-02T15:04:05Z"
	confirmYes = "yes"
)

var providerEnvVars = map[string]string{
	"amazon-bedrock":         "AWS_BEARER_TOKEN_BEDROCK",
	"anthropic":              "ANTHROPIC_API_KEY",
	"azure-openai-responses": "AZURE_OPENAI_API_KEY",
	"cerebras":               "CEREBRAS_API_KEY",
	"github-copilot":         "GITHUB_TOKEN",
	"google":                 "GEMINI_API_KEY",
	"google-antigravity":     "GEMINI_API_KEY",
	"google-gemini-cli":      "GEMINI_API_KEY",
	"google-vertex":          "GOOGLE_APPLICATION_CREDENTIALS",
	"groq":                   "GROQ_API_KEY",
	"huggingface":            "HF_TOKEN",
	"kimi-coding":            "KIMI_API_KEY",
	"minimax":                "MINIMAX_API_KEY",
	"minimax-cn":             "MINIMAX_CN_API_KEY",
	"mistral":                "MISTRAL_API_KEY",
	"openai":                 "OPENAI_API_KEY",
	"openai-codex":           "OPENAI_API_KEY",
	"opencode":               "OPENCODE_API_KEY",
	"opencode-zen":           "OPENCODE_API_KEY",
	"openrouter":             "OPENROUTER_API_KEY",
	"vercel-ai-gateway":      "AI_GATEWAY_API_KEY",
	"xai":                    "XAI_API_KEY",
	"zai":                    "ZAI_API_KEY",
}

var rootCmd = &cobra.Command{
	Use:   "ply [provider|id] [args...]",
	Short: "A CLI tool for managing AI providers",
	Long:  `Ply is a command-line tool for managing and configuring AI provider configurations for pi coding agent.`,
	Run:   runRoot,
	Args:  cobra.ArbitraryArgs,
}

func Execute() error {
	return rootCmd.Execute()
}

func init() {
	rootCmd.CompletionOptions.DisableDefaultCmd = false
}

func runRoot(cmd *cobra.Command, args []string) {
	dataDir, err := fs.DataDir()
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error getting data directory: %v\n", err)
		os.Exit(1)
	}

	db := database.New(dataDir)
	if err := db.Load(context.Background()); err != nil {
		fmt.Fprintf(os.Stderr, "Error loading database: %v\n", err)
		os.Exit(1)
	}

	providerArg, piArgs, skipModelsFilter := parseArgs(args)

	entry, err := resolveEntry(db, providerArg)
	if err != nil {
		os.Exit(1)
	}

	masterKey, err := fs.LoadMasterKey()
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error loading master key: %v\n", err)
		fmt.Fprintf(os.Stderr, "Run 'ply setup' to initialize ply.\n")
		os.Exit(1)
	}

	apiKey, err := crypto.Decrypt(masterKey, entry.Cipher, entry.Nonce)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error decrypting API key: %v\n", err)
		os.Exit(1)
	}
	defer apiKey.Zero()

	envVar, ok := providerEnvVar(entry.Provider)
	if !ok {
		fmt.Fprintf(os.Stderr, "Error: unsupported provider '%s'\n", entry.Provider)
		os.Exit(1)
	}

	if err := os.Setenv(envVar, apiKey.String()); err != nil {
		fmt.Fprintf(os.Stderr, "Error setting environment variable: %v\n", err)
		os.Exit(1)
	}

	executePi(entry, piArgs, skipModelsFilter)
}

func parseArgs(args []string) (providerArg string, piArgs []string, skipModelsFilter bool) {
	for i, arg := range args {
		if arg == "--" {
			skipModelsFilter = true
			if i > 0 {
				providerArg = args[0]
			}
			piArgs = args[i+1:]
			return
		}
	}

	if len(args) > 0 {
		if len(args[0]) > 0 && args[0][0] == '-' {
			piArgs = args
		} else {
			providerArg = args[0]
			piArgs = args[1:]
		}
	}
	return
}

func resolveEntry(db *database.Database, providerArg string) (database.Entry, error) {
	if providerArg != "" {
		entry, err := db.GetEntryByLabel(providerArg)
		if err != nil {
			entry, err = db.GetEntry(providerArg)
			if err != nil {
				fmt.Fprintf(os.Stderr, "Error: provider '%s' not found\n", providerArg)
				fmt.Fprintf(os.Stderr, "Use 'ply config list' to see all configured providers.\n")
				return database.Entry{}, err
			}
		}
		return entry, nil
	}

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
		return database.Entry{}, err
	}

	entry, err := db.GetEntry(defaultID)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error: default provider entry not found (ID: %s)\n", defaultID)
		fmt.Fprintf(os.Stderr, "The default provider may have been deleted.\n")
		fmt.Fprintf(os.Stderr, "Use 'ply default [label|id]' to set a new default provider.\n")
		return database.Entry{}, err
	}
	return entry, nil
}

func executePi(entry database.Entry, piArgs []string, skipModelsFilter bool) {
	var finalPiArgs []string
	if !skipModelsFilter {
		if entry.DefaultModel != "" {
			finalPiArgs = []string{"--provider", entry.Provider, "--model", entry.DefaultModel}
		} else {
			finalPiArgs = []string{"--models", fmt.Sprintf("%s/*", entry.Provider)}
		}
	}
	finalPiArgs = append(finalPiArgs, piArgs...)

	piCmd := exec.Command("pi", finalPiArgs...)
	piCmd.Stdin = os.Stdin
	piCmd.Stdout = os.Stdout
	piCmd.Stderr = os.Stderr

	if err := piCmd.Run(); err != nil {
		if exitErr, ok := err.(*exec.ExitError); ok {
			os.Exit(exitErr.ExitCode())
		}
		fmt.Fprintf(os.Stderr, "Error running pi: %v\n", err)
		os.Exit(1)
	}
}

func providerEnvVar(provider string) (string, bool) {
	envVar, ok := providerEnvVars[provider]
	return envVar, ok
}
