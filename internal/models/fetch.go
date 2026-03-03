package models

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"os"
	"time"
)

const (
	// DefaultGitHub configuration - can be overridden via environment variables
	defaultGitHubAPIURL   = "https://api.github.com"
	defaultGitHubRawURL   = "https://raw.githubusercontent.com"
	defaultOwner          = "badlogic"
	defaultRepo           = "pi-mono"
	defaultModelsFilePath = "packages/ai/src/models.generated.ts"

	// Environment variables for configuration
	envGitHubAPIURL   = "PLY_GITHUB_API_URL"
	envGitHubRawURL   = "PLY_GITHUB_RAW_URL"
	envOwner          = "PLY_PI_MONO_OWNER"
	envRepo           = "PLY_PI_MONO_REPO"
	envModelsFilePath = "PLY_MODELS_FILE_PATH"
)

// GitHubConfig holds the configuration for fetching models from GitHub.
// This allows decoupling from the hardcoded pi-mono repository.
type GitHubConfig struct {
	APIURL     string
	RawURL     string
	Owner      string
	Repo       string
	ModelsPath string
	UserAgent  string
}

// DefaultGitHubConfig returns a GitHubConfig with default values.
// Environment variables can override defaults.
func DefaultGitHubConfig() *GitHubConfig {
	cfg := &GitHubConfig{
		APIURL:     defaultGitHubAPIURL,
		RawURL:     defaultGitHubRawURL,
		Owner:      defaultOwner,
		Repo:       defaultRepo,
		ModelsPath: defaultModelsFilePath,
		UserAgent:  "ply-cli",
	}

	// Override with environment variables if set
	if v := os.Getenv(envGitHubAPIURL); v != "" {
		cfg.APIURL = v
	}
	if v := os.Getenv(envGitHubRawURL); v != "" {
		cfg.RawURL = v
	}
	if v := os.Getenv(envOwner); v != "" {
		cfg.Owner = v
	}
	if v := os.Getenv(envRepo); v != "" {
		cfg.Repo = v
	}
	if v := os.Getenv(envModelsFilePath); v != "" {
		cfg.ModelsPath = v
	}

	return cfg
}

// Shared HTTP client with reasonable timeouts
var httpClient = &http.Client{
	Timeout: 30 * time.Second,
}

type ReleaseResponse struct {
	TagName string `json:"tag_name"`
}

// FetchLatestReleaseTag fetches the latest release tag from the configured GitHub repository.
func FetchLatestReleaseTag(ctx context.Context) (string, error) {
	cfg := DefaultGitHubConfig()
	return fetchLatestReleaseTagWithConfig(ctx, cfg)
}

func fetchLatestReleaseTagWithConfig(ctx context.Context, cfg *GitHubConfig) (string, error) {
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, fmt.Sprintf("%s/repos/%s/%s/releases/latest", cfg.APIURL, cfg.Owner, cfg.Repo), nil)
	if err != nil {
		return "", fmt.Errorf("failed to create request: %w", err)
	}

	req.Header.Set("Accept", "application/vnd.github+json")
	req.Header.Set("User-Agent", cfg.UserAgent)

	resp, err := httpClient.Do(req)
	if err != nil {
		return "", fmt.Errorf("failed to fetch release: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return "", fmt.Errorf("unexpected status code: %d", resp.StatusCode)
	}

	var release ReleaseResponse
	if err := json.NewDecoder(resp.Body).Decode(&release); err != nil {
		return "", fmt.Errorf("failed to decode release: %w", err)
	}

	return release.TagName, nil
}

// FetchModelsFile fetches the models file from the configured GitHub repository.
func FetchModelsFile(ctx context.Context, tag string) (string, error) {
	cfg := DefaultGitHubConfig()
	return fetchModelsFileWithConfig(ctx, cfg, tag)
}

func fetchModelsFileWithConfig(ctx context.Context, cfg *GitHubConfig, tag string) (string, error) {
	url := fmt.Sprintf("%s/%s/%s/%s/%s", cfg.RawURL, cfg.Owner, cfg.Repo, tag, cfg.ModelsPath)

	req, err := http.NewRequestWithContext(ctx, http.MethodGet, url, nil)
	if err != nil {
		return "", fmt.Errorf("failed to create request: %w", err)
	}

	req.Header.Set("Accept", "application/vnd.github+json")
	req.Header.Set("User-Agent", cfg.UserAgent)

	resp, err := httpClient.Do(req)
	if err != nil {
		return "", fmt.Errorf("failed to fetch models file: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return "", fmt.Errorf("unexpected status code: %d", resp.StatusCode)
	}

	data, err := io.ReadAll(resp.Body)
	if err != nil {
		return "", fmt.Errorf("failed to read response: %w", err)
	}

	return string(data), nil
}

// FetchLatest fetches the latest models from the configured GitHub repository.
func FetchLatest(ctx context.Context) (Models, string, error) {
	cfg := DefaultGitHubConfig()
	return fetchLatestWithConfig(ctx, cfg)
}

func fetchLatestWithConfig(ctx context.Context, cfg *GitHubConfig) (Models, string, error) {
	tag, err := fetchLatestReleaseTagWithConfig(ctx, cfg)
	if err != nil {
		return nil, "", fmt.Errorf("failed to get latest release: %w", err)
	}

	content, err := fetchModelsFileWithConfig(ctx, cfg, tag)
	if err != nil {
		return nil, "", fmt.Errorf("failed to fetch models: %w", err)
	}

	models, err := ParseModels(content)
	if err != nil {
		return nil, "", fmt.Errorf("failed to parse models: %w", err)
	}

	return models, tag, nil
}

func FetchAndCache(ctx context.Context) error {
	cfg := DefaultGitHubConfig()
	models, tag, err := fetchLatestWithConfig(ctx, cfg)
	if err != nil {
		// Fall back to cached models if available
		cache, loadErr := LoadCache()
		if loadErr == nil && cache.Models != nil {
			return nil // Use cached models silently
		}
		return err
	}

	if err := SaveCache(models, tag); err != nil {
		return fmt.Errorf("failed to save cache: %w", err)
	}

	return nil
}

func GetModels(ctx context.Context) (Models, error) {
	cache, err := LoadCache()
	if err == nil && !cache.IsStale() {
		return cache.Models, nil
	}

	cfg := DefaultGitHubConfig()
	models, _, err := fetchLatestWithConfig(ctx, cfg)
	if err != nil {
		if cache != nil {
			return cache.Models, nil
		}
		return nil, fmt.Errorf("failed to fetch models: %w", err)
	}

	if err := SaveCache(models, ""); err != nil {
		fmt.Fprintf(os.Stderr, "Warning: failed to cache models: %v\n", err)
	}

	return models, nil
}
