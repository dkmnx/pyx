package settings

import (
	"os"
	"path/filepath"
	"testing"
)

func TestDefaultSettings(t *testing.T) {
	s := DefaultSettings()

	if s.GitHub.APIURL != "https://api.github.com" {
		t.Errorf("GitHub.APIURL = %q, want %q", s.GitHub.APIURL, "https://api.github.com")
	}
	if s.GitHub.Owner != "badlogic" {
		t.Errorf("GitHub.Owner = %q, want %q", s.GitHub.Owner, "badlogic")
	}
	if s.GitHub.Repo != "pi-mono" {
		t.Errorf("GitHub.Repo = %q, want %q", s.GitHub.Repo, "pi-mono")
	}
	if len(s.CustomProviderEnvVars) != 0 {
		t.Errorf("CustomProviderEnvVars = %v, want empty map", s.CustomProviderEnvVars)
	}
}

func TestLoad_NoSettingsFile(t *testing.T) {
	// Temporarily change home directory
	origHome := os.Getenv("HOME")
	tmpDir := t.TempDir()
	os.Setenv("HOME", tmpDir)
	defer os.Setenv("HOME", origHome)

	s, err := Load()
	if err != nil {
		t.Fatalf("Load() error = %v", err)
	}

	if s.GitHub.Owner != "badlogic" {
		t.Errorf("GitHub.Owner = %q, want %q", s.GitHub.Owner, "badlogic")
	}
}

func TestLoad_WithSettingsFile(t *testing.T) {
	origHome := os.Getenv("HOME")
	tmpDir := t.TempDir()
	os.Setenv("HOME", tmpDir)
	defer os.Setenv("HOME", origHome)

	// Create settings file
	settingsDir := filepath.Join(tmpDir, ".local", "share", "ply")
	if err := os.MkdirAll(settingsDir, 0700); err != nil {
		t.Fatalf("Failed to create settings dir: %v", err)
	}

	settingsContent := `{
		"github": {
			"owner": "custom-owner",
			"repo": "custom-repo"
		},
		"customProviderEnvVars": {
			"custom-provider": "CUSTOM_API_KEY"
		}
	}`

	settingsPath := filepath.Join(settingsDir, "settings.json")
	if err := os.WriteFile(settingsPath, []byte(settingsContent), 0600); err != nil {
		t.Fatalf("Failed to write settings file: %v", err)
	}

	s, err := Load()
	if err != nil {
		t.Fatalf("Load() error = %v", err)
	}

	if s.GitHub.Owner != "custom-owner" {
		t.Errorf("GitHub.Owner = %q, want %q", s.GitHub.Owner, "custom-owner")
	}
	if s.GitHub.Repo != "custom-repo" {
		t.Errorf("GitHub.Repo = %q, want %q", s.GitHub.Repo, "custom-repo")
	}
	if envVar, ok := s.CustomProviderEnvVars["custom-provider"]; !ok || envVar != "CUSTOM_API_KEY" {
		t.Errorf("CustomProviderEnvVars[custom-provider] = %q, want %q", envVar, "CUSTOM_API_KEY")
	}
}

func TestSave(t *testing.T) {
	origHome := os.Getenv("HOME")
	tmpDir := t.TempDir()
	os.Setenv("HOME", tmpDir)
	defer os.Setenv("HOME", origHome)

	s := DefaultSettings()
	s.GitHub.Owner = "test-owner"
	s.CustomProviderEnvVars["test-provider"] = "TEST_API_KEY"

	if err := s.Save(); err != nil {
		t.Fatalf("Save() error = %v", err)
	}

	// Load it back
	loaded, err := Load()
	if err != nil {
		t.Fatalf("Load() error = %v", err)
	}

	if loaded.GitHub.Owner != "test-owner" {
		t.Errorf("Loaded GitHub.Owner = %q, want %q", loaded.GitHub.Owner, "test-owner")
	}
	if envVar, ok := loaded.CustomProviderEnvVars["test-provider"]; !ok || envVar != "TEST_API_KEY" {
		t.Errorf("Loaded CustomProviderEnvVars[test-provider] = %q, want %q", envVar, "TEST_API_KEY")
	}
}

func TestDataDir(t *testing.T) {
	origHome := os.Getenv("HOME")
	tmpDir := t.TempDir()
	os.Setenv("HOME", tmpDir)
	defer os.Setenv("HOME", origHome)

	dir, err := DataDir()
	if err != nil {
		t.Fatalf("DataDir() error = %v", err)
	}

	expected := filepath.Join(tmpDir, ".local", "share", "ply")
	if dir != expected {
		t.Errorf("DataDir() = %q, want %q", dir, expected)
	}
}
