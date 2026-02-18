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
	"github.com/dkmnx/ply/internal/keys"
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
	Long:  `Ply is a command-line tool for managing and configuring AI provider configurations for pi coding agent.`,
	Run:   runRoot,
	Args:  cobra.ArbitraryArgs,
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
	dataDir, err := fs.DataDir()
	if err != nil {
		fatal(err)
	}

	db := database.New(dataDir)
	if err := db.Load(context.Background()); err != nil {
		fatal(err)
	}

	providerArg, piArgs, skipModelsFilter := parseArgs(args)

	entries, err := resolveEntries(db, providerArg)
	if err != nil {
		os.Exit(1)
	}

	// Initialize key manager
	keyMgr := keys.New(dataDir)

	// Check if master key exists
	keyExists, err := keyMgr.Exists()
	if err != nil {
		fatal(err)
	}

	if !keyExists {
		fmt.Fprintln(os.Stderr, "Master key not found.")
		fmt.Fprintln(os.Stderr, "Run 'ply setup' to initialize ply.")
		os.Exit(1)
	}

	// Check if password is required
	requiresPassword, err := keyMgr.RequiresPassword()
	if err != nil {
		fatal(err)
	}

	var password []byte
	if requiresPassword {
		fmt.Print("Enter password to unlock your API keys: ")
		pwStr, err := prompt.ReadPassword()
		if err != nil {
			fatal(err)
		}
		fmt.Println()
		password = []byte(pwStr)
	}

	// Load master key
	masterKey, err := keyMgr.Load(password)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error loading master key: %v\n", err)
		if err == keys.ErrInvalidPassword {
			fmt.Fprintln(os.Stderr, "Password incorrect.")
		}
		os.Exit(1)
	}

	providerEnv, err := buildProviderEnv(masterKey, entries)
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}

	executePi(entries, piArgs, skipModelsFilter, providerEnv, sessionFlag)
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
		if exitErr, ok := err.(*exec.ExitError); ok {
			os.Exit(exitErr.ExitCode())
		}
		fatalf("Error running pi: %v", err)
	}

	// Display hint after pi exits (only if no session flag was provided)
	if sessionFlag == "" {
		displaySessionHint()
	}
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

	fmt.Fprintf(os.Stderr, "\n")
	fmt.Fprintf(os.Stderr, "To continue this session, run: ply -s %s\n", sessionInfo.UUID)
	fmt.Fprintf(os.Stderr, "\n")
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
	env := os.Environ()
	for envVar, value := range envValues {
		env = append(env, envVar+"="+string(value.Bytes()))
		// Zero the SecureString after use
		value.Zero()
	}

	return env, nil
}
