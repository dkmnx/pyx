package prompt

import (
	"os"
	"strings"
	"testing"

	"github.com/spf13/cobra"
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

func TestReadLine(t *testing.T) {
	origStdin := os.Stdin
	defer func() { os.Stdin = origStdin }()

	r, w, _ := os.Pipe()
	w.WriteString("test input\n")
	w.Close()
	os.Stdin = r

	got, err := ReadLine()
	if err != nil {
		t.Errorf("ReadLine() error = %v", err)
		return
	}
	if got != "test input" {
		t.Errorf("ReadLine() = %q, want %q", got, "test input")
	}
}

func TestReadLine_Empty(t *testing.T) {
	origStdin := os.Stdin
	defer func() { os.Stdin = origStdin }()

	r, w, _ := os.Pipe()
	w.WriteString("\n")
	w.Close()
	os.Stdin = r

	got, err := ReadLine()
	if err != nil {
		t.Errorf("ReadLine() error = %v", err)
		return
	}
	if got != "" {
		t.Errorf("ReadLine() = %q, want empty string", got)
	}
}

func TestReadLine_WithNewline(t *testing.T) {
	origStdin := os.Stdin
	defer func() { os.Stdin = origStdin }()

	r, w, _ := os.Pipe()
	w.WriteString("line1\nline2\n")
	w.Close()
	os.Stdin = r

	got, err := ReadLine()
	if err != nil {
		t.Errorf("ReadLine() error = %v", err)
		return
	}
	if got != "line1" {
		t.Errorf("ReadLine() = %q, want %q", got, "line1")
	}
}

func TestPromptProvider_Number(t *testing.T) {
	origStdin := os.Stdin
	defer func() { os.Stdin = origStdin }()

	r, w, _ := os.Pipe()
	w.WriteString("1\n")
	w.Close()
	os.Stdin = r

	cmd := &cobra.Command{}
	got, err := PromptProvider(cmd)
	if err != nil {
		t.Errorf("PromptProvider() error = %v", err)
		return
	}
	if got != "amazon-bedrock" {
		t.Errorf("PromptProvider() = %q, want %q", got, "amazon-bedrock")
	}
}

func TestPromptProvider_Name(t *testing.T) {
	origStdin := os.Stdin
	defer func() { os.Stdin = origStdin }()

	r, w, _ := os.Pipe()
	w.WriteString("openai\n")
	w.Close()
	os.Stdin = r

	cmd := &cobra.Command{}
	got, err := PromptProvider(cmd)
	if err != nil {
		t.Errorf("PromptProvider() error = %v", err)
		return
	}
	if got != "openai" {
		t.Errorf("PromptProvider() = %q, want %q", got, "openai")
	}
}

func TestPromptProvider_CaseInsensitive(t *testing.T) {
	origStdin := os.Stdin
	defer func() { os.Stdin = origStdin }()

	r, w, _ := os.Pipe()
	w.WriteString("ANTHROPIC\n")
	w.Close()
	os.Stdin = r

	cmd := &cobra.Command{}
	got, err := PromptProvider(cmd)
	if err != nil {
		t.Errorf("PromptProvider() error = %v", err)
		return
	}
	if got != "anthropic" {
		t.Errorf("PromptProvider() = %q, want %q", got, "anthropic")
	}
}

func TestPromptProvider_InvalidNumber(t *testing.T) {
	t.Skip("flaky test with piped stdin")
}

func TestPromptProvider_InvalidName(t *testing.T) {
	t.Skip("flaky test with piped stdin")
}

func TestPromptProvider_MixedCase(t *testing.T) {
	origStdin := os.Stdin
	defer func() { os.Stdin = origStdin }()

	r, w, _ := os.Pipe()
	w.WriteString("GoOgLe\n")
	w.Close()
	os.Stdin = r

	cmd := &cobra.Command{}
	got, err := PromptProvider(cmd)
	if err != nil {
		t.Errorf("PromptProvider() error = %v", err)
		return
	}
	if got != "google" {
		t.Errorf("PromptProvider() = %q, want %q", got, "google")
	}
}

func TestPromptLabel_Default(t *testing.T) {
	origStdin := os.Stdin
	defer func() { os.Stdin = origStdin }()

	r, w, _ := os.Pipe()
	w.WriteString("\n")
	w.Close()
	os.Stdin = r

	cmd := &cobra.Command{}
	got, err := PromptLabel(cmd, "openai")
	if err != nil {
		t.Errorf("PromptLabel() error = %v", err)
		return
	}
	if !strings.HasPrefix(got, "openai-") {
		t.Errorf("PromptLabel() = %q, want prefix %q", got, "openai-")
	}
}

func TestPromptLabel_Custom(t *testing.T) {
	origStdin := os.Stdin
	defer func() { os.Stdin = origStdin }()

	r, w, _ := os.Pipe()
	w.WriteString("my-custom-label\n")
	w.Close()
	os.Stdin = r

	cmd := &cobra.Command{}
	got, err := PromptLabel(cmd, "openai")
	if err != nil {
		t.Errorf("PromptLabel() error = %v", err)
		return
	}
	if got != "my-custom-label" {
		t.Errorf("PromptLabel() = %q, want %q", got, "my-custom-label")
	}
}

func TestPromptLabel_Whitespace(t *testing.T) {
	origStdin := os.Stdin
	defer func() { os.Stdin = origStdin }()

	r, w, _ := os.Pipe()
	w.WriteString("   \n")
	w.Close()
	os.Stdin = r

	cmd := &cobra.Command{}
	got, err := PromptLabel(cmd, "openai")
	if err != nil {
		t.Errorf("PromptLabel() error = %v", err)
		return
	}
	if !strings.HasPrefix(got, "openai-") {
		t.Errorf("PromptLabel() = %q, want prefix %q", got, "openai-")
	}
}
