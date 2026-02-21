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
	githubAPIURL   = "https://api.github.com"
	githubRawURL   = "https://raw.githubusercontent.com"
	owner          = "badlogic"
	repo           = "pi-mono"
	modelsFilePath = "packages/ai/src/models.generated.ts"
)

// Shared HTTP client with reasonable timeouts
var httpClient = &http.Client{
	Timeout: 30 * time.Second,
}

type ReleaseResponse struct {
	TagName string `json:"tag_name"`
}

func FetchLatestReleaseTag(ctx context.Context) (string, error) {
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, fmt.Sprintf("%s/repos/%s/%s/releases/latest", githubAPIURL, owner, repo), nil)
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

func FetchModelsFile(ctx context.Context, tag string) (string, error) {
	url := fmt.Sprintf("%s/%s/%s/%s/%s", githubRawURL, owner, repo, tag, modelsFilePath)

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

func FetchLatest(ctx context.Context) (Models, string, error) {
	tag, err := FetchLatestReleaseTag(ctx)
	if err != nil {
		return nil, "", fmt.Errorf("failed to get latest release: %w", err)
	}

	content, err := FetchModelsFile(ctx, tag)
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
	models, tag, err := FetchLatest(ctx)
	if err != nil {
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

	models, _, err := FetchLatest(ctx)
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
