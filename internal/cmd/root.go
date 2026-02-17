package cmd

import (
	"context"
	"fmt"
	"os"
	"os/exec"
	"sort"
	"strings"

	"github.com/dkmnx/ply/internal/crypto"
	"github.com/dkmnx/ply/internal/database"
	"github.com/dkmnx/ply/internal/fs"
	"github.com/dkmnx/ply/internal/providers"
	"github.com/spf13/cobra"
)

const (
	timeFormat = "2006-01-02T15:04:05Z"
	confirmYes = "yes"
)

var rootCmd = &cobra.Command{
	Use:   "ply [provider] [args...]",
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

	entries, err := resolveEntries(db, providerArg)
	if err != nil {
		os.Exit(1)
	}

	masterKey, err := fs.LoadMasterKey()
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error loading master key: %v\n", err)
		fmt.Fprintf(os.Stderr, "Run 'ply setup' to initialize ply.\n")
		os.Exit(1)
	}

	if err := setProviderEnvVars(masterKey, entries); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}

	executePi(entries, piArgs, skipModelsFilter)
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

func resolveEntries(db *database.Database, providerArg string) ([]database.Entry, error) {
	if providerArg != "" {
		entry, err := db.GetEntry(providerArg)
		if err != nil {
			fmt.Fprintf(os.Stderr, "Error: provider '%s' not found\n", providerArg)
			fmt.Fprintf(os.Stderr, "Use 'ply config list' to see all configured providers.\n")
			return nil, err
		}
		return []database.Entry{entry}, nil
	}

	entries := db.ListEntries()
	if len(entries) == 0 {
		fmt.Fprintln(os.Stderr, "No providers configured.")
		fmt.Fprintln(os.Stderr, "Run 'ply setup' to add a provider.")
		return nil, fmt.Errorf("no providers configured")
	}

	sort.Slice(entries, func(i, j int) bool {
		return entries[i].Provider < entries[j].Provider
	})

	return entries, nil
}

func executePi(entries []database.Entry, piArgs []string, skipModelsFilter bool) {
	var finalPiArgs []string
	if !skipModelsFilter {
		providersList := make([]string, 0, len(entries))
		for _, entry := range entries {
			providersList = append(providersList, fmt.Sprintf("%s/*", entry.Provider))
		}
		finalPiArgs = []string{"--models", strings.Join(providersList, ",")}
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
	return providers.EnvVar(provider)
}

func setProviderEnvVars(masterKey []byte, entries []database.Entry) error {
	envValues := make(map[string]string)
	for _, entry := range entries {
		apiKey, err := crypto.Decrypt(masterKey, entry.Cipher, entry.Nonce)
		if err != nil {
			return fmt.Errorf("Error decrypting API key for provider '%s': %w", entry.Provider, err)
		}
		decrypted := apiKey.String()
		apiKey.Zero()

		envVar, ok := providerEnvVar(entry.Provider)
		if !ok {
			return fmt.Errorf("Error: unsupported provider '%s'", entry.Provider)
		}

		if existing, ok := envValues[envVar]; ok {
			if existing != decrypted {
				return fmt.Errorf("Conflicting API keys: provider '%s' has a different key for %s", entry.Provider, envVar)
			}
			continue
		}
		envValues[envVar] = decrypted
	}

	for envVar, value := range envValues {
		if err := os.Setenv(envVar, value); err != nil {
			return fmt.Errorf("Error setting environment variable %s: %w", envVar, err)
		}
	}

	return nil
}
