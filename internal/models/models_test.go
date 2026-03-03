package models

import (
	"os"
	"testing"
	"time"
)

func TestParseModels(t *testing.T) {
	tests := []struct {
		name    string
		content string
		want    Models
		wantErr bool
	}{
		{
			name: "valid content with provider field",
			content: `
model "foo" {
  provider: "anthropic"
  id: "claude-3-5-sonnet-20241022"
}
`,
			want: Models{
				"anthropic": {"claude-3-5-sonnet-20241022"},
			},
			wantErr: false,
		},
		{
			name: "multiple models same provider",
			content: `
model "foo" {
  provider: "google"
  id: "gemini-1.5-pro"
}
model "bar" {
  provider: "google"
  id: "gemini-1.5-flash"
}
`,
			want: Models{
				"google": {"gemini-1.5-pro", "gemini-1.5-flash"},
			},
			wantErr: false,
		},
		{
			name: "ignores comments",
			content: `
// this is a comment
model "foo" {
  provider: "anthropic"
  id: "claude-3"
}
`,
			want: Models{
				"anthropic": {"claude-3"},
			},
			wantErr: false,
		},
		{
			name: "empty content",
			content: `
`,
			want:    nil,
			wantErr: true,
		},
		{
			name: "duplicates are deduplicated",
			content: `
model "foo" {
  provider: "openai"
  id: "gpt-4"
}
model "bar" {
  provider: "openai"
  id: "gpt-4"
}
`,
			want: Models{
				"openai": {"gpt-4"},
			},
			wantErr: false,
		},
		{
			name: "unknown provider with valid models",
			content: `
model "foo" {
  provider: "unknown-provider"
  id: "model-1"
}
model "bar" {
  provider: "anthropic"
  id: "claude-3"
}
`,
			want: Models{
				"unknown-provider": {"model-1"},
				"anthropic":        {"claude-3"},
			},
			wantErr: false,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got, err := ParseModels(tt.content)
			if (err != nil) != tt.wantErr {
				t.Errorf("ParseModels() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			if tt.wantErr {
				return
			}
			if len(got) != len(tt.want) {
				t.Errorf("ParseModels() got %v, want %v", got, tt.want)
				return
			}
			for provider, models := range tt.want {
				if len(got[provider]) != len(models) {
					t.Errorf("ParseModels() provider %s got %v, want %v", provider, got[provider], models)
				}
			}
		})
	}
}

func TestCacheIsStale(t *testing.T) {
	tests := []struct {
		name      string
		updatedAt time.Time
		want      bool
	}{
		{
			name:      "fresh cache",
			updatedAt: time.Now().UTC(),
			want:      false,
		},
		{
			name:      "stale cache",
			updatedAt: time.Now().UTC().Add(-48 * time.Hour),
			want:      true,
		},
		{
			name:      "just under TTL",
			updatedAt: time.Now().UTC().Add(-23 * time.Hour),
			want:      false,
		},
		{
			name:      "just over TTL",
			updatedAt: time.Now().UTC().Add(-25 * time.Hour),
			want:      true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			c := &Cache{
				Version:   "v1.0.0",
				UpdatedAt: tt.updatedAt,
				Models:    Models{},
			}
			got := c.IsStale()
			if got != tt.want {
				t.Errorf("IsStale() = %v, want %v", got, tt.want)
			}
		})
	}
}

func TestCacheRoundTrip(t *testing.T) {
	origXDG := os.Getenv("XDG_DATA_HOME")
	tmpDir := t.TempDir()
	os.Setenv("XDG_DATA_HOME", tmpDir)
	defer os.Setenv("XDG_DATA_HOME", origXDG)

	models := Models{
		"anthropic": {"claude-3-sonnet", "claude-3-opus"},
		"openai":    {"gpt-4", "gpt-4-turbo"},
	}

	err := SaveCache(models, "v1.0.0")
	if err != nil {
		t.Fatalf("SaveCache() error = %v", err)
	}

	loaded, err := LoadCache()
	if err != nil {
		t.Fatalf("LoadCache() error = %v", err)
	}

	if loaded.Version != "v1.0.0" {
		t.Errorf("LoadCache().Version = %v, want v1.0.0", loaded.Version)
	}

	if len(loaded.Models) != len(models) {
		t.Errorf("LoadCache().Models = %v, want %v", loaded.Models, models)
	}

	for provider, modelList := range models {
		if len(loaded.Models[provider]) != len(modelList) {
			t.Errorf("LoadCache().Models[%s] = %v, want %v", provider, loaded.Models[provider], modelList)
		}
	}

	version, err := LoadCacheVersion()
	if err != nil {
		t.Fatalf("LoadCacheVersion() error = %v", err)
	}
	if version != "v1.0.0" {
		t.Errorf("LoadCacheVersion() = %v, want v1.0.0", version)
	}
}

func TestCacheLoadNoCache(t *testing.T) {
	origXDG := os.Getenv("XDG_DATA_HOME")
	tmpDir := t.TempDir()
	os.Setenv("XDG_DATA_HOME", tmpDir)
	defer os.Setenv("XDG_DATA_HOME", origXDG)

	_, err := LoadCache()
	if err != ErrNoCache {
		t.Errorf("LoadCache() error = %v, want ErrNoCache", err)
	}
}

func TestCacheClearCache(t *testing.T) {
	origXDG := os.Getenv("XDG_DATA_HOME")
	tmpDir := t.TempDir()
	os.Setenv("XDG_DATA_HOME", tmpDir)
	defer os.Setenv("XDG_DATA_HOME", origXDG)

	models := Models{"openai": {"gpt-4"}}
	if err := SaveCache(models, "v1.0.0"); err != nil {
		t.Fatalf("SaveCache() error = %v", err)
	}

	if !CacheExists() {
		t.Error("CacheExists() = false, want true after SaveCache")
	}

	if err := ClearCache(); err != nil {
		t.Fatalf("ClearCache() error = %v", err)
	}

	if CacheExists() {
		t.Error("CacheExists() = true, want false after ClearCache")
	}
}

func TestCacheExists(t *testing.T) {
	origXDG := os.Getenv("XDG_DATA_HOME")
	tmpDir := t.TempDir()
	os.Setenv("XDG_DATA_HOME", tmpDir)
	defer os.Setenv("XDG_DATA_HOME", origXDG)

	if CacheExists() {
		t.Error("CacheExists() = true, want false when no cache exists")
	}

	models := Models{"openai": {"gpt-4"}}
	if err := SaveCache(models, "v1.0.0"); err != nil {
		t.Fatalf("SaveCache() error = %v", err)
	}

	if !CacheExists() {
		t.Error("CacheExists() = false, want true after SaveCache")
	}
}

func TestDefaultGitHubConfig(t *testing.T) {
	// Test default values
	cfg := DefaultGitHubConfig()
	if cfg.APIURL != defaultGitHubAPIURL {
		t.Errorf("APIURL = %q, want %q", cfg.APIURL, defaultGitHubAPIURL)
	}
	if cfg.RawURL != defaultGitHubRawURL {
		t.Errorf("RawURL = %q, want %q", cfg.RawURL, defaultGitHubRawURL)
	}
	if cfg.Owner != defaultOwner {
		t.Errorf("Owner = %q, want %q", cfg.Owner, defaultOwner)
	}
	if cfg.Repo != defaultRepo {
		t.Errorf("Repo = %q, want %q", cfg.Repo, defaultRepo)
	}
	if cfg.ModelsPath != defaultModelsFilePath {
		t.Errorf("ModelsPath = %q, want %q", cfg.ModelsPath, defaultModelsFilePath)
	}
	if cfg.UserAgent != "ply-cli" {
		t.Errorf("UserAgent = %q, want %q", cfg.UserAgent, "ply-cli")
	}
}

func TestDefaultGitHubConfig_EnvOverrides(t *testing.T) {
	// Save original env vars
	origEnv := map[string]string{
		envGitHubAPIURL:   os.Getenv(envGitHubAPIURL),
		envGitHubRawURL:   os.Getenv(envGitHubRawURL),
		envOwner:          os.Getenv(envOwner),
		envRepo:           os.Getenv(envRepo),
		envModelsFilePath: os.Getenv(envModelsFilePath),
	}
	defer func() {
		// Restore original env vars
		for k, v := range origEnv {
			if v == "" {
				os.Unsetenv(k)
			} else {
				os.Setenv(k, v)
			}
		}
	}()

	// Set test env vars
	os.Setenv(envGitHubAPIURL, "https://api.github.enterprise.com")
	os.Setenv(envGitHubRawURL, "https://raw.github.enterprise.com")
	os.Setenv(envOwner, "custom-owner")
	os.Setenv(envRepo, "custom-repo")
	os.Setenv(envModelsFilePath, "custom/path/models.ts")

	cfg := DefaultGitHubConfig()

	if cfg.APIURL != "https://api.github.enterprise.com" {
		t.Errorf("APIURL = %q, want %q", cfg.APIURL, "https://api.github.enterprise.com")
	}
	if cfg.RawURL != "https://raw.github.enterprise.com" {
		t.Errorf("RawURL = %q, want %q", cfg.RawURL, "https://raw.github.enterprise.com")
	}
	if cfg.Owner != "custom-owner" {
		t.Errorf("Owner = %q, want %q", cfg.Owner, "custom-owner")
	}
	if cfg.Repo != "custom-repo" {
		t.Errorf("Repo = %q, want %q", cfg.Repo, "custom-repo")
	}
	if cfg.ModelsPath != "custom/path/models.ts" {
		t.Errorf("ModelsPath = %q, want %q", cfg.ModelsPath, "custom/path/models.ts")
	}
}
