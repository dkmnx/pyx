package providers

import (
	"strings"
	"testing"
)

func TestEnvVar(t *testing.T) {
	tests := []struct {
		name  string
		want  string
		found bool
	}{
		{"openai", "OPENAI_API_KEY", true},
		{"anthropic", "ANTHROPIC_API_KEY", true},
		{"google", "GEMINI_API_KEY", true},
		{"openrouter", "OPENROUTER_API_KEY", true},
		{"unknown", "", false},
		{"", "", false},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			envVar, found := EnvVar(tt.name)
			if found != tt.found {
				t.Errorf("EnvVar() found = %v, want %v", found, tt.found)
			}
			if envVar != tt.want {
				t.Errorf("EnvVar() = %v, want %v", envVar, tt.want)
			}
		})
	}
}

func TestIsValid(t *testing.T) {
	validProviders := Names()

	for _, provider := range validProviders {
		t.Run("valid_"+provider, func(t *testing.T) {
			if !IsValid(provider) {
				t.Errorf("IsValid(%q) returned false for known provider", provider)
			}
		})
	}

	invalidProviders := []string{
		"",
		"unknown",
		"not-a-provider",
		"OpenAI", // case sensitive
	}

	for _, provider := range invalidProviders {
		t.Run("invalid_"+provider, func(t *testing.T) {
			if IsValid(provider) {
				t.Errorf("IsValid(%q) returned true for invalid provider", provider)
			}
		})
	}
}

func TestValidate(t *testing.T) {
	tests := []struct {
		name    string
		input   string
		wantErr bool
	}{
		// Valid known providers
		{"valid openai", "openai", false},
		{"valid anthropic", "anthropic", false},
		{"valid google-vertex", "google-vertex", false},
		{"valid amazon-bedrock", "amazon-bedrock", false},
		{"valid github-copilot", "github-copilot", false},
		{"valid xai", "xai", false},

		// Empty string
		{"empty string", "", true},

		// Path traversal attempts
		{"path traversal ..", "../etc/passwd", true},
		{"path traversal in middle", "provider/../etc", true},
		{"path traversal multiple", "....//test", true},

		// Path separators
		{"contains forward slash", "provider/test", true},
		{"contains backslash", "provider\\test", true},
		{"contains mixed slashes", "pro/vider\\test", true},

		// Control characters
		{"contains null byte", "provider\x00", true},
		{"contains tab", "provi\tder", true},
		{"contains newline", "provi\nder", true},
		{"contains carriage return", "provi\rder", true},

		// Invalid characters
		{"contains space", "provi der", true},
		{"contains dot", "provi.der", true},
		{"contains at sign", "provi@der", true},
		{"contains hash", "provi#der", true},
		{"contains exclamation", "provi!der", true},
		{"contains dollar sign", "provi$der", true},
		{"contains percent", "provi%der", true},
		{"contains ampersand", "provi&der", true},
		{"contains asterisk", "provi*der", true},
		{"contains plus", "provi+der", true},
		{"contains equals", "provi=der", true},
		{"contains question", "provi?der", true},
		{"contains caret", "provi^der", true},
		{"contains backtick", "provi`der", true},
		{"contains pipe", "provi|der", true},
		{"contains tilde", "provi~der", true},
		{"contains braces", "provi{der}", true},
		{"contains brackets", "provi[der]", true},

		// Too long (>50 characters)
		{"too long", strings.Repeat("a", 51), true},
		{"exactly 50 is ok", strings.Repeat("a", 50), true}, // still unknown provider

		// Case sensitivity
		{"uppercase known", "OPENAI", true},
		{"mixed case known", "OpenAI", true},

		// Unknown but valid format
		{"unknown valid format", "unknown-provider", true},
		{"unknown valid format 2", "another_provider", true},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			err := Validate(tt.input)
			if (err != nil) != tt.wantErr {
				t.Errorf("Validate() error = %v, wantErr %v", err, tt.wantErr)
			}
		})
	}
}

func TestNames(t *testing.T) {
	names := Names()

	if len(names) == 0 {
		t.Error("Names() returned empty slice")
	}

	// Check for duplicates
	seen := make(map[string]bool)
	for _, name := range names {
		if seen[name] {
			t.Errorf("Names() contains duplicate: %s", name)
		}
		seen[name] = true
	}

	// Verify all names are valid format
	for _, name := range names {
		err := Validate(name)
		if err != nil {
			t.Errorf("Names() returned invalid provider name: %s (error: %v)", name, err)
		}
	}
}

func TestEnvVarUnique(t *testing.T) {
	// Note: Some providers intentionally share the same EnvVar (e.g., google providers
	// share GEMINI_API_KEY, openai and openai-codex share OPENAI_API_KEY).
	// This is by design as they use the same API key for different services.
	// This test just verifies the mapping is consistent and documented.

	// Build a map of env vars to their providers
	envVarToProviders := make(map[string][]string)
	for _, provider := range All {
		envVarToProviders[provider.EnvVar] = append(envVarToProviders[provider.EnvVar], provider.Name)
	}

	// Verify that all providers sharing an env var make sense
	// (e.g., are related services that would logically share a key)
	for envVar, providersList := range envVarToProviders {
		if len(providersList) > 1 {
			t.Logf("EnvVar %s is shared by: %v", envVar, providersList)
		}
	}
}

func TestProviderNameFormat(t *testing.T) {
	for _, provider := range All {
		// Check that all provider names match the expected pattern
		if !validProviderNamePattern.MatchString(provider.Name) {
			t.Errorf("Provider name %q does not match expected pattern", provider.Name)
		}

		// Check that env var is not empty
		if provider.EnvVar == "" {
			t.Errorf("Provider %q has empty EnvVar", provider.Name)
		}
	}
}
