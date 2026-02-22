package cmd

import (
	"context"
	"errors"
	"fmt"
	"os"
	"os/exec"
	"sort"
	"strings"

	"github.com/dkmnx/ply/internal/crypto"
	"github.com/dkmnx/ply/internal/database"
	"github.com/dkmnx/ply/internal/fs"
	"github.com/dkmnx/ply/internal/keys"
	"github.com/dkmnx/ply/internal/pi"
	"github.com/dkmnx/ply/internal/prompt"
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
	// Initialize key manager, database, and ensure master key exists
	keyMgr, db, err := initializeKeyManager()
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}

	// Load database
	if err := db.Load(context.Background()); err != nil {
		fmt.Fprintf(os.Stderr, "Error loading database: %v\n", err)
		os.Exit(1)
	}

	// Parse arguments and resolve entries
	providerArg, piArgs, skipModelsFilter := parseArgs(args)

	entries, err := resolveEntries(db, providerArg)
	if err != nil {
		os.Exit(1)
	}

	// Load master key (prompting for password if needed)
	masterKey, err := loadMasterKey(keyMgr)
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}

	// Ensure master key is zeroed after use
	defer zeroMasterKey(masterKey)

	// Build provider environment and execute pi
	providerEnv, err := buildProviderEnv(masterKey, entries)
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}

	executePi(entries, piArgs, skipModelsFilter, providerEnv, sessionFlag)
}

// initializeKeyManager creates the key manager, database, handles migration, and verifies master key exists.
func initializeKeyManager() (*keys.Manager, *database.Database, error) {
	dataDir, err := fs.DataDir()
	if err != nil {
		return nil, nil, err
	}

	keyMgr := keys.New(dataDir)
	db := database.New(dataDir)

	// Attempt to migrate from legacy master key file if it exists
	_, err = keyMgr.MigrateFromLegacy(db)
	if err != nil {
		return nil, nil, fmt.Errorf("migrating master key: %w", err)
	}

	// Check if master key exists
	keyExists, err := keyMgr.Exists()
	if err != nil {
		return nil, nil, err
	}

	if !keyExists {
		return nil, nil, errors.New("master key not found, run 'ply init' to initialize")
	}

	return keyMgr, db, nil
}

// loadMasterKey prompts for password if needed and loads the master key.
func loadMasterKey(keyMgr *keys.Manager) ([]byte, error) {
	requiresPassword, err := keyMgr.RequiresPassword()
	if err != nil {
		return nil, fmt.Errorf("Error checking password requirement: %w", err)
	}

	var password []byte
	if requiresPassword {
		pwStr, err := prompt.PromptPassword(context.Background(), "Enter password to unlock your API keys")
		if err != nil {
			return nil, fmt.Errorf("Error reading password: %w", err)
		}
		password = []byte(pwStr)
	}

	masterKey, err := keyMgr.Load(password)
	if err != nil {
		if err == keys.ErrInvalidPassword {
			return nil, errors.New("password incorrect")
		}
		return nil, fmt.Errorf("loading master key: %w", err)
	}

	return masterKey, nil
}

// zeroMasterKey securely zeros the master key bytes.
func zeroMasterKey(masterKey []byte) {
	for i := range masterKey {
		masterKey[i] = 0
	}
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

func executePi(entries []database.Entry, piArgs []string, skipModelsFilter bool, providerEnv []string, sessionFlag string) {
	// Check if pi is installed, auto-install if not
	wasInstalled, err := pi.EnsureInstalled()
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
	if skipModelsFilter {
		finalPiArgs = piArgs
	} else {
		providersList := make([]string, 0, len(entries))
		for _, entry := range entries {
			providersList = append(providersList, fmt.Sprintf("%s/*", entry.Provider))
		}
		finalPiArgs = []string{"--models", strings.Join(providersList, ",")}
		finalPiArgs = append(finalPiArgs, piArgs...)
	}

	// Add session flag if provided
	if sessionFlag != "" {
		finalPiArgs = append(finalPiArgs, "--session", sessionFlag)
	}

	piCmd := exec.Command("pi", finalPiArgs...)
	piCmd.Stdin = os.Stdin
	piCmd.Stdout = os.Stdout
	piCmd.Stderr = os.Stderr
	piCmd.Env = providerEnv

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

func buildProviderEnv(masterKey []byte, entries []database.Entry) ([]string, error) {
	envValues := make(map[string]*crypto.SecureString)
	for _, entry := range entries {
		apiKey, err := crypto.Decrypt(masterKey, entry.Cipher, entry.Nonce)
		if err != nil {
			return nil, fmt.Errorf("Error decrypting API key for provider '%s': %w", entry.Provider, err)
		}
		// Create SecureString directly from SecureBytes without intermediate string
		decrypted := crypto.NewSecureStringFromBytes(apiKey)

		envVar, ok := providerEnvVar(entry.Provider)
		if !ok {
			decrypted.Zero()
			return nil, fmt.Errorf("Error: unsupported provider '%s'", entry.Provider)
		}

		if existing, ok := envValues[envVar]; ok {
			if !existing.Equal(decrypted) {
				decrypted.Zero()
				return nil, fmt.Errorf("Conflicting API keys: provider '%s' has a different key for %s", entry.Provider, envVar)
			}
			decrypted.Zero()
			continue
		}
		envValues[envVar] = decrypted
	}

	// Build environment slice: start with current process env, then add/override with provider env
	//
	// SECURITY NOTE: This function necessarily exposes decrypted API keys as environment
	// variables to the spawned subprocess. This is a fundamental limitation of passing
	// environment variables to OS processes via exec.Cmd - they must be strings.
	//
	// Mitigations in place:
	//   - API keys are decrypted using crypto.SecureString which zeros memory on destruction
	//   - The SecureString is zeroed immediately after string conversion (value.Zero())
	//   - Keys are only decrypted on-demand when pi is executed, not stored in memory
	//
	// This risk is inherent to any CLI tool that passes secrets to subprocesses. Users should
	// ensure their environment is secure (e.g., not running on shared systems).
	env := os.Environ()
	for envVar, value := range envValues {
		env = append(env, envVar+"="+string(value.Bytes()))
		// Zero the SecureString after use
		value.Zero()
	}

	return env, nil
}
