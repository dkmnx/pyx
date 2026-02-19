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
		{"google-vertex", "GOOGLE_APPLICATION_CREDENTIALS", true},
		{"openrouter", "OPENROUTER_API_KEY", true},
		{"groq", "GROQ_API_KEY", true},
		{"mistral", "MISTRAL_API_KEY", true},
		{"xai", "XAI_API_KEY", true},
		{"zai", "ZAI_API_KEY", true},
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

func TestDeriveEnvVar(t *testing.T) {
	tests := []struct {
		name     string
		provider string
		want     string
	}{
		{"cohere provider", "cohere", "COHERE_API_KEY"},
		{"deepseek provider", "deepseek", "DEEPSEEK_API_KEY"},
		{"meta provider", "meta", "META_API_KEY"},
		{"nvidia provider", "nvidia", "NVIDIA_API_KEY"},
		{"moonshot provider", "moonshot", "MOONSHOT_API_KEY"},
		{"qwen provider", "qwen", "QWEN_API_KEY"},
		{"writer provider", "writer", "WRITER_API_KEY"},
		{"google vertex", "google-vertex", "GOOGLE_APPLICATION_CREDENTIALS"},
		{"custom -ai suffix", "custom-ai", "CUSTOM_API_KEY"},
		{"unknown provider", "unknown-provider", "UNKNOWN-PROVIDER_API_KEY"},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got, _ := deriveEnvVar(tt.provider)
			if got != tt.want {
				t.Errorf("deriveEnvVar() = %v, want %v", got, tt.want)
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
		// Valid format (might not be in cache, but format is valid)
		{"valid openai", "openai", false},
		{"valid anthropic", "anthropic", false},
		{"valid google-vertex", "google-vertex", false},
		{"valid amazon-bedrock", "amazon-bedrock", false},
		{"valid github-copilot", "github-copilot", false},
		{"valid xai", "xai", false},
		{"valid minimax", "minimax", false},

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

		// Exactly 50 is valid format
		{"exactly 50", strings.Repeat("a", 50), false},
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

	// Note: This test may return empty if cache is not available
	// That's expected behavior
	if len(names) == 0 {
		t.Skip("No models cache available, skipping Names() test")
		return
	}

	// Check for duplicates
	seen := make(map[string]bool)
	for _, name := range names {
		if seen[name] {
			t.Errorf("Names() contains duplicate: %s", name)
		}
		seen[name] = true
	}

	// Verify all names have valid format
	for _, name := range names {
		if !validProviderNamePattern.MatchString(name) {
			t.Errorf("Names() returned invalid provider name format: %s", name)
		}
	}
}

func TestEnvVarMappingCoverage(t *testing.T) {
	// Test that we have env vars for known providers
	knownProviders := []string{
		"openai", "anthropic", "google", "groq", "mistral",
		"amazon-bedrock", "azure-openai-responses", "cerebras",
		"github-copilot", "huggingface", "kimi-coding",
		"minimax", "minimax-cn", "openai-codex", "opencode",
		"openrouter", "vercel-ai-gateway", "xai", "zai",
	}

	for _, provider := range knownProviders {
		t.Run("has_env_var_"+provider, func(t *testing.T) {
			envVar, found := EnvVar(provider)
			if !found {
				t.Errorf("EnvVar(%q) returned false for known provider", provider)
			}
			if envVar == "" {
				t.Errorf("EnvVar(%q) returned empty string", provider)
			}
		})
	}
}
