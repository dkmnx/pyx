// Package settings manages ply configuration settings.
package settings

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
)

const (
	settingsFileName = "settings.json"
	defaultOwner     = "badlogic"
	defaultRepo      = "pi-mono"
)

// Settings holds all ply configuration settings.
type Settings struct {
	// GitHub configuration for fetching models
	GitHub *GitHubSettings `json:"github,omitempty"`

	// Custom provider environment variable mappings
	// Key is provider name, value is environment variable name
	CustomProviderEnvVars map[string]string `json:"customProviderEnvVars,omitempty"`
}

// GitHubSettings holds GitHub-specific configuration.
type GitHubSettings struct {
	APIURL     string `json:"apiUrl,omitempty"`
	RawURL     string `json:"rawUrl,omitempty"`
	Owner      string `json:"owner,omitempty"`
	Repo       string `json:"repo,omitempty"`
	ModelsPath string `json:"modelsPath,omitempty"`
}

// Load loads settings from the configuration file.
// Returns default settings if file doesn't exist.
func Load() (*Settings, error) {
	settingsPath, err := FilePath()
	if err != nil {
		return DefaultSettings(), fmt.Errorf("failed to get settings path: %w", err)
	}

	data, err := os.ReadFile(settingsPath)
	if err != nil {
		if os.IsNotExist(err) {
			return DefaultSettings(), nil
		}
		return nil, fmt.Errorf("failed to read settings file: %w", err)
	}

	var settings Settings
	if err := json.Unmarshal(data, &settings); err != nil {
		return nil, fmt.Errorf("failed to parse settings file: %w", err)
	}

	// Apply defaults for nil fields
	if settings.GitHub == nil {
		settings.GitHub = &GitHubSettings{}
	}
	if settings.CustomProviderEnvVars == nil {
		settings.CustomProviderEnvVars = make(map[string]string)
	}

	// Fill in defaults from environment variables or hardcoded defaults
	settings.applyDefaults()

	return &settings, nil
}

// Save saves settings to the configuration file.
func (s *Settings) Save() error {
	settingsPath, err := FilePath()
	if err != nil {
		return fmt.Errorf("failed to get settings path: %w", err)
	}

	// Ensure directory exists
	if err := os.MkdirAll(filepath.Dir(settingsPath), 0700); err != nil {
		return fmt.Errorf("failed to create settings directory: %w", err)
	}

	data, err := json.MarshalIndent(s, "", "  ")
	if err != nil {
		return fmt.Errorf("failed to marshal settings: %w", err)
	}

	// Write with secure permissions
	if err := os.WriteFile(settingsPath, data, 0600); err != nil {
		return fmt.Errorf("failed to write settings file: %w", err)
	}

	return nil
}

// applyDefaults fills in default values for unset fields.
// Priority: Settings file > Environment variables > Hardcoded defaults
func (s *Settings) applyDefaults() {
	if s.GitHub == nil {
		s.GitHub = &GitHubSettings{}
	}

	// GitHub API URL
	if s.GitHub.APIURL == "" {
		if env := os.Getenv("PLY_GITHUB_API_URL"); env != "" {
			s.GitHub.APIURL = env
		} else {
			s.GitHub.APIURL = "https://api.github.com"
		}
	}

	// GitHub Raw URL
	if s.GitHub.RawURL == "" {
		if env := os.Getenv("PLY_GITHUB_RAW_URL"); env != "" {
			s.GitHub.RawURL = env
		} else {
			s.GitHub.RawURL = "https://raw.githubusercontent.com"
		}
	}

	// Owner
	if s.GitHub.Owner == "" {
		if env := os.Getenv("PLY_PI_MONO_OWNER"); env != "" {
			s.GitHub.Owner = env
		} else {
			s.GitHub.Owner = defaultOwner
		}
	}

	// Repo
	if s.GitHub.Repo == "" {
		if env := os.Getenv("PLY_PI_MONO_REPO"); env != "" {
			s.GitHub.Repo = env
		} else {
			s.GitHub.Repo = defaultRepo
		}
	}

	// Models Path
	if s.GitHub.ModelsPath == "" {
		if env := os.Getenv("PLY_MODELS_FILE_PATH"); env != "" {
			s.GitHub.ModelsPath = env
		} else {
			s.GitHub.ModelsPath = "packages/ai/src/models.generated.ts"
		}
	}
}

// DefaultSettings returns settings with all default values.
func DefaultSettings() *Settings {
	return &Settings{
		GitHub: &GitHubSettings{
			APIURL:     "https://api.github.com",
			RawURL:     "https://raw.githubusercontent.com",
			Owner:      defaultOwner,
			Repo:       defaultRepo,
			ModelsPath: "packages/ai/src/models.generated.ts",
		},
		CustomProviderEnvVars: make(map[string]string),
	}
}

// FilePath returns the path to the settings file.
func FilePath() (string, error) {
	dataDir, err := os.UserHomeDir()
	if err != nil {
		return "", err
	}
	return filepath.Join(dataDir, ".local", "share", "ply", settingsFileName), nil
}

// DataDir returns the ply data directory.
func DataDir() (string, error) {
	homeDir, err := os.UserHomeDir()
	if err != nil {
		return "", err
	}
	return filepath.Join(homeDir, ".local", "share", "ply"), nil
}
