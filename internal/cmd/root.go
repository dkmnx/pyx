package cmd

import (
	"context"
	"errors"
	"fmt"
	"os"
	"os/exec"
	"sort"

	"github.com/dkmnx/ply/internal/crypto"
	"github.com/dkmnx/ply/internal/database"
	"github.com/dkmnx/ply/internal/fs"
	"github.com/dkmnx/ply/internal/keys"
	"github.com/dkmnx/ply/internal/pi"
	"github.com/dkmnx/ply/internal/providers"
	"github.com/dkmnx/ply/internal/session"
	"github.com/spf13/cobra"
)

const (
	timeFormat = "2006-01-02T15:04:05Z"
	confirmYes = "yes"
	confirmY   = "y"
)

var rootCmd = &cobra.Command{
	Use:   "ply [provider] [args...]",
	Short: "A CLI tool for managing AI providers",
	Long: `Ply is a command-line tool for managing and configuring AI provider configurations for pi coding agent.

Pi will be auto-installed if not found, or you can install manually with: 'ply pi install'`,
	Run:  runRoot,
	Args: cobra.ArbitraryArgs,
}

var sessionFlag string

func Execute() error {
	return rootCmd.Execute()
}

func init() {
	rootCmd.CompletionOptions.DisableDefaultCmd = false
	rootCmd.Flags().StringVarP(&sessionFlag, "session", "s", "", "Session UUID to pass to pi")
}

func runRoot(cmd *cobra.Command, args []string) {
	ctx := cmd.Context()
	if ctx == nil {
		ctx = context.Background()
	}

	// Initialize key manager, database, and ensure master key exists
	keyMgr, db, err := initializeKeyManager()
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}

	// Load database
	if err := db.Load(ctx); err != nil {
		fmt.Fprintf(os.Stderr, "Error loading database: %v\n", err)
		os.Exit(1)
	}

	// Parse arguments and resolve entries
	selectedProvider, piCommandArgs := parseArgs(args)

	providerEntries, err := resolveEntries(db, selectedProvider)
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}

	// Load master key (prompting for password if needed)
	masterKey, err := loadMasterKey(keyMgr)
	if err != nil {
		fmt.Fprintln(os.Stderr, "Could not decrypt your API keys. Run 'ply setup' to recreate your configuration.")
		os.Exit(1)
	}

	// Ensure master key is zeroed after use
	defer zeroMasterKey(masterKey)

	// Build provider environment and execute pi
	providerEnvVars, err := buildProviderEnv(masterKey, providerEntries)
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}

	executePi(providerEntries, piCommandArgs, providerEnvVars, sessionFlag)
}

// initializeKeyManager creates the key manager and database, and verifies master key exists.
func initializeKeyManager() (*keys.Manager, *database.Database, error) {
	dataDir, err := fs.DataDir()
	if err != nil {
		return nil, nil, err
	}

	keyMgr := keys.New(dataDir)
	db := database.New(dataDir)

	// Check if master key exists
	keyExists, err := keyMgr.Exists()
	if err != nil {
		return nil, nil, err
	}

	if !keyExists {
		return nil, nil, errors.New("master key not found, run 'ply setup' to initialize")
	}

	return keyMgr, db, nil
}

func loadMasterKey(keyMgr *keys.Manager) ([]byte, error) {
	masterKey, err := keyMgr.Load(nil)
	if err != nil {
		return nil, err
	}
	return masterKey, nil
}

// zeroMasterKey securely zeros the master key bytes.
func zeroMasterKey(masterKey []byte) {
	for i := range masterKey {
		masterKey[i] = 0
	}
}

func parseArgs(args []string) (providerArg string, piArgs []string) {
	for i, arg := range args {
		if arg == "--" {
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

// validateProvider checks if a provider name is valid against pi's model list.
// Returns a helpful error message if validation fails.
func validateProvider(provider string) error {
	return providers.Validate(provider)
}

func resolveEntries(db *database.Database, selectedProvider string) ([]database.Entry, error) {
	if selectedProvider != "" {
		// Validate provider name before looking up in database
		if err := validateProvider(selectedProvider); err != nil {
			return nil, err
		}

		entry, err := db.GetEntry(selectedProvider)
		if err != nil {
			return nil, fmt.Errorf("provider '%s' not found. Use 'ply list' to see all configured providers", selectedProvider)
		}
		return []database.Entry{entry}, nil
	}

	providerEntries := db.ListEntries()
	if len(providerEntries) == 0 {
		return nil, fmt.Errorf("no providers configured. Run 'ply setup' to add a provider")
	}

	sort.Slice(providerEntries, func(i, j int) bool {
		return providerEntries[i].Provider < providerEntries[j].Provider
	})

	return providerEntries, nil
}

func executePi(providerEntries []database.Entry, piCommandArgs []string, providerEnvVars []string, sessionFlag string) {
	// Check if pi is installed, auto-install if not
	wasInstalled, err := pi.EnsureInstalled(context.Background())
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error checking pi installation: %v\n", err)
		fmt.Fprintln(os.Stderr, "Please install pi manually:")
		fmt.Fprintf(os.Stderr, "  %s\n", pi.InstallCommand())
		os.Exit(1)
	}

	if wasInstalled {
		// pi was just installed, show platform info and install completion
		fmt.Fprintf(os.Stderr, "Platform: %s\n", pi.PlatformInfo())
		if version, err := pi.Version(); err == nil {
			fmt.Fprintf(os.Stderr, "pi version: %s\n", version)
		}
		fmt.Fprintln(os.Stderr)

		// Install shell completion for detected shell
		if err := pi.InstallCompletion(); err != nil {
			fmt.Fprintf(os.Stderr, "Warning: failed to install shell completions: %v\n", err)
		}
	}

	var finalPiArgs []string
	finalPiArgs = piCommandArgs

	// Add session flag if provided
	if sessionFlag != "" {
		finalPiArgs = append(finalPiArgs, "--session", sessionFlag)
	}

	piCmd := exec.Command("pi", finalPiArgs...)
	piCmd.Stdin = os.Stdin
	piCmd.Stdout = os.Stdout
	piCmd.Stderr = os.Stderr
	piCmd.Env = providerEnvVars

	if err := piCmd.Run(); err != nil {
		var exitErr *exec.ExitError
		if errors.As(err, &exitErr) {
			os.Exit(exitErr.ExitCode())
		}
		fmt.Fprintf(os.Stderr, "Error running pi: %v\n", err)
		os.Exit(1)
	}

	// Display hint after pi exits
	displaySessionHint()
}

// displaySessionHint shows a hint about how to resume the session
func displaySessionHint() {
	cwd, err := os.Getwd()
	if err != nil {
		return
	}

	sessionDir, err := session.DirForCwd(cwd)
	if err != nil {
		return
	}

	sessionInfo, err := session.FindMostRecentSession(sessionDir)
	if err != nil || sessionInfo == nil {
		return
	}

	// ANSI color code for gray (bright black)
	gray := "\033[90m"
	reset := "\033[0m"

	fmt.Fprintf(os.Stderr, "  ██████  ██%s\n", gray)
	fmt.Fprintf(os.Stderr, "  %s██  ██  ██%s  To continue this session, run:\n", reset, gray)
	fmt.Fprintf(os.Stderr, "  %s████  ██    ply -s %s\n", reset, sessionInfo.UUID)
	fmt.Fprintf(os.Stderr, "  ██    ██\n\n")
}

func providerEnvVar(provider string) (string, bool) {
	return providers.EnvVar(provider)
}

// decryptProviderKeys decrypts API keys for all configured providers.
func decryptProviderKeys(masterKey []byte, providerEntries []database.Entry) (map[string]*crypto.SecureString, error) {
	decryptedAPIKeys := make(map[string]*crypto.SecureString)

	for _, entry := range providerEntries {
		apiKey, err := crypto.Decrypt(string(masterKey), entry.Cipher)
		if err != nil {
			// Zero any already-decrypted keys before returning
			for _, v := range decryptedAPIKeys {
				v.Zero()
			}
			return nil, fmt.Errorf("error decrypting API key for provider '%s': %w", entry.Provider, err)
		}
		decrypted := crypto.NewSecureStringFromBytes(apiKey)
		decryptedAPIKeys[entry.Provider] = decrypted
	}

	return decryptedAPIKeys, nil
}

// mapProvidersToEnvVars maps providers to their environment variables.
// Returns (envVars, error) where envVars is mapping of env var names to values.
func mapProvidersToEnvVars(decryptedAPIKeys map[string]*crypto.SecureString) (map[string]*crypto.SecureString, error) {
	envVarToAPIKey := make(map[string]*crypto.SecureString)

	for provider, apiKey := range decryptedAPIKeys {
		envVar, ok := providerEnvVar(provider)
		if !ok {
			apiKey.Zero()
			return nil, fmt.Errorf("unsupported provider '%s'", provider)
		}

		if existing, ok := envVarToAPIKey[envVar]; ok {
			if !existing.Equal(apiKey) {
				apiKey.Zero()
				existing.Zero()
				return nil, fmt.Errorf("conflicting API keys: provider '%s' has a different key for %s", provider, envVar)
			}
			apiKey.Zero()
			continue
		}
		envVarToAPIKey[envVar] = apiKey
	}

	return envVarToAPIKey, nil
}

// buildEnvSlice builds the final environment variable slice.
// It zeroes all SecureStrings after use.
func buildEnvSlice(envVarToAPIKey map[string]*crypto.SecureString) []string {
	env := os.Environ()
	for envVar, apiKey := range envVarToAPIKey {
		env = append(env, envVar+"="+string(apiKey.Bytes()))
		apiKey.Zero()
	}
	return env
}

func buildProviderEnv(masterKey []byte, providerEntries []database.Entry) ([]string, error) {
	// Decrypt all provider keys
	decryptedAPIKeys, err := decryptProviderKeys(masterKey, providerEntries)
	if err != nil {
		if errors.Is(err, crypto.ErrInvalidPassphrase) {
			return nil, errors.New("could not decrypt your API keys. Run 'ply setup' to recreate your configuration")
		}
		return nil, err
	}

	// Map providers to environment variables
	envVarToAPIKey, err := mapProvidersToEnvVars(decryptedAPIKeys)
	if err != nil {
		return nil, err
	}

	// Build final environment slice
	return buildEnvSlice(envVarToAPIKey), nil
}
