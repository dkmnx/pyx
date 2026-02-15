package prompt

import (
	"strings"
	"testing"
)

func TestProviders(t *testing.T) {
	tests := []struct {
		name     string
		provider string
		want     bool
	}{
		{"amazon-bedrock exists", "amazon-bedrock", true},
		{"anthropic exists", "anthropic", true},
		{"azure-openai-responses exists", "azure-openai-responses", true},
		{"cerebras exists", "cerebras", true},
		{"github-copilot exists", "github-copilot", true},
		{"google exists", "google", true},
		{"google-antigravity exists", "google-antigravity", true},
		{"google-gemini-cli exists", "google-gemini-cli", true},
		{"google-vertex exists", "google-vertex", true},
		{"groq exists", "groq", true},
		{"huggingface exists", "huggingface", true},
		{"kimi-coding exists", "kimi-coding", true},
		{"minimax exists", "minimax", true},
		{"minimax-cn exists", "minimax-cn", true},
		{"mistral exists", "mistral", true},
		{"openai exists", "openai", true},
		{"openai-codex exists", "openai-codex", true},
		{"opencode exists", "opencode", true},
		{"openrouter exists", "openrouter", true},
		{"vercel-ai-gateway exists", "vercel-ai-gateway", true},
		{"xai exists", "xai", true},
		{"zai exists", "zai", true},
		{"unknown provider", "unknown", false},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			found := false
			for _, p := range Providers {
				if p == tt.provider {
					found = true
					break
				}
			}
			if found != tt.want {
				t.Errorf("provider %s: found=%v, want=%v", tt.provider, found, tt.want)
			}
		})
	}
}

func TestProviderCount(t *testing.T) {
	expected := 22
	if len(Providers) != expected {
		t.Errorf("Providers count = %d, want %d", len(Providers), expected)
	}
}

func TestRandomSuffix(t *testing.T) {
	for i := 0; i < 100; i++ {
		suffix := randomSuffix(4)
		if len(suffix) != 8 {
			t.Errorf("randomSuffix(4) = %v, want length 8", suffix)
		}

		for _, c := range suffix {
			if !((c >= '0' && c <= '9') || (c >= 'a' && c <= 'f')) {
				t.Errorf("randomSuffix() contains invalid char %c", c)
				break
			}
		}
	}
}

func TestRandomSuffixUniqueness(t *testing.T) {
	suffixes := make(map[string]bool)
	for i := 0; i < 100; i++ {
		suffix := randomSuffix(4)
		if suffixes[suffix] {
			t.Logf("Warning: duplicate suffix generated: %s", suffix)
		}
		suffixes[suffix] = true
	}

	if len(suffixes) < 90 {
		t.Errorf("randomSuffix() generated too many duplicates: %d unique out of 100", len(suffixes))
	}
}

func TestProviderValidation(t *testing.T) {
	validProviders := []string{
		"amazon-bedrock", "anthropic", "azure-openai-responses", "cerebras",
		"github-copilot", "google", "google-antigravity", "google-gemini-cli",
		"google-vertex", "groq", "huggingface", "kimi-coding",
		"minimax", "minimax-cn", "mistral", "openai", "openai-codex",
		"opencode", "openrouter", "vercel-ai-gateway", "xai", "zai",
	}

	for _, provider := range validProviders {
		found := false
		for _, p := range Providers {
			if p == provider {
				found = true
				break
			}
		}
		if !found {
			t.Errorf("valid provider %s not found in Providers list", provider)
		}
	}
}

func TestProviderCaseInsensitive(t *testing.T) {
	tests := []struct {
		name     string
		input    string
		expected string
	}{
		{"uppercase", "ANTHROPIC", "anthropic"},
		{"mixed case", "OpEnAi", "openai"},
		{"lowercase", "google", "google"},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			for _, p := range Providers {
				if strings.EqualFold(tt.input, p) {
					if p != tt.expected {
						t.Errorf("case-insensitive match for %s found %s, want %s", tt.input, p, tt.expected)
					}
					return
				}
			}
			t.Errorf("no match found for %s", tt.input)
		})
	}
}
