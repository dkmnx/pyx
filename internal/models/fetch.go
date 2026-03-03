package models

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"os"
	"time"

	"github.com/dkmnx/ply/internal/settings"
)

// Shared HTTP client with reasonable timeouts
var httpClient = &http.Client{
	Timeout: 30 * time.Second,
}

type ReleaseResponse struct {
	TagName string `json:"tag_name"`
}

// FetchLatestReleaseTag fetches the latest release tag from the configured GitHub repository.
func FetchLatestReleaseTag(ctx context.Context) (string, error) {
	cfg, err := loadGitHubConfig()
	if err != nil {
		return "", err
	}
	return fetchLatestReleaseTagWithConfig(ctx, cfg)
}

func fetchLatestReleaseTagWithConfig(ctx context.Context, cfg *settings.GitHubSettings) (string, error) {
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, fmt.Sprintf("%s/repos/%s/%s/releases/latest", cfg.APIURL, cfg.Owner, cfg.Repo), nil)
	if err != nil {
		return "", fmt.Errorf("failed to create request: %w", err)
	}

	req.Header.Set("Accept", "application/vnd.github+json")
	req.Header.Set("User-Agent", "ply-cli")

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
	cfg, err := loadGitHubConfig()
	if err != nil {
		return "", err
	}
	return fetchModelsFileWithConfig(ctx, cfg, tag)
}

func fetchModelsFileWithConfig(ctx context.Context, cfg *settings.GitHubSettings, tag string) (string, error) {
	url := fmt.Sprintf("%s/%s/%s/%s/%s", cfg.RawURL, cfg.Owner, cfg.Repo, tag, cfg.ModelsPath)

	req, err := http.NewRequestWithContext(ctx, http.MethodGet, url, nil)
	if err != nil {
		return "", fmt.Errorf("failed to create request: %w", err)
	}

	req.Header.Set("Accept", "application/vnd.github+json")
	req.Header.Set("User-Agent", "ply-cli")

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
	cfg, err := loadGitHubConfig()
	if err != nil {
		return nil, "", err
	}
	return fetchLatestWithConfig(ctx, cfg)
}

func fetchLatestWithConfig(ctx context.Context, cfg *settings.GitHubSettings) (Models, string, error) {
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
	cfg, err := loadGitHubConfig()
	if err != nil {
		return err
	}

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

	cfg, err := loadGitHubConfig()
	if err != nil {
		return nil, err
	}

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

// loadGitHubConfig loads the GitHub configuration from settings.
func loadGitHubConfig() (*settings.GitHubSettings, error) {
	s, err := settings.Load()
	if err != nil {
		return nil, fmt.Errorf("failed to load settings: %w", err)
	}
	return s.GitHub, nil
}
