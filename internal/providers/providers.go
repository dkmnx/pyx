package providers

import (
	"fmt"
	"regexp"
	"strings"
)

type Provider struct {
	Name   string
	EnvVar string
}

var (
	// validProviderNamePattern matches valid provider names.
	// Allows alphanumeric, hyphens, and underscores, 1-50 characters.
	validProviderNamePattern = regexp.MustCompile(`^[a-zA-Z0-9_-]{1,50}$`)
)

var All = []Provider{
	{Name: "amazon-bedrock", EnvVar: "AWS_BEARER_TOKEN_BEDROCK"},
	{Name: "anthropic", EnvVar: "ANTHROPIC_API_KEY"},
	{Name: "azure-openai-responses", EnvVar: "AZURE_OPENAI_API_KEY"},
	{Name: "cerebras", EnvVar: "CEREBRAS_API_KEY"},
	{Name: "github-copilot", EnvVar: "GITHUB_TOKEN"},
	{Name: "google", EnvVar: "GEMINI_API_KEY"},
	{Name: "google-antigravity", EnvVar: "GEMINI_API_KEY"},
	{Name: "google-gemini-cli", EnvVar: "GEMINI_API_KEY"},
	{Name: "google-vertex", EnvVar: "GOOGLE_APPLICATION_CREDENTIALS"},
	{Name: "groq", EnvVar: "GROQ_API_KEY"},
	{Name: "huggingface", EnvVar: "HF_TOKEN"},
	{Name: "kimi-coding", EnvVar: "KIMI_API_KEY"},
	{Name: "minimax", EnvVar: "MINIMAX_API_KEY"},
	{Name: "minimax-cn", EnvVar: "MINIMAX_CN_API_KEY"},
	{Name: "mistral", EnvVar: "MISTRAL_API_KEY"},
	{Name: "openai", EnvVar: "OPENAI_API_KEY"},
	{Name: "openai-codex", EnvVar: "OPENAI_API_KEY"},
	{Name: "opencode", EnvVar: "OPENCODE_API_KEY"},
	{Name: "opencode-zen", EnvVar: "OPENCODE_API_KEY"},
	{Name: "openrouter", EnvVar: "OPENROUTER_API_KEY"},
	{Name: "vercel-ai-gateway", EnvVar: "AI_GATEWAY_API_KEY"},
	{Name: "xai", EnvVar: "XAI_API_KEY"},
	{Name: "zai", EnvVar: "ZAI_API_KEY"},
}

// Names returns a list of all supported provider names.
func Names() []string {
	names := make([]string, len(All))
	for i, p := range All {
		names[i] = p.Name
	}
	return names
}

// EnvVar returns the environment variable name required for the given provider.
// Returns the env var name and true if the provider is found.
// Returns empty string and false if the provider is not recognized.
func EnvVar(name string) (string, bool) {
	for _, p := range All {
		if p.Name == name {
			return p.EnvVar, true
		}
	}
	return "", false
}

// IsValid returns true if the given provider name is recognized.
// This does not validate the format of the name, only checks against
// the list of supported providers.
func IsValid(name string) bool {
	_, ok := EnvVar(name)
	return ok
}

// Validate checks if a provider name is valid and safe.
// It performs validation beyond just checking against the known provider list.
func Validate(name string) error {
	// Check for empty string
	if name == "" {
		return fmt.Errorf("provider name cannot be empty")
	}

	// Check for path traversal attempts
	if strings.Contains(name, "..") {
		return fmt.Errorf("provider name cannot contain '..' (path traversal attempt)")
	}

	// Check for path separators
	if strings.ContainsAny(name, "/\\") {
		return fmt.Errorf("provider name cannot contain path separators")
	}

	// Check for control characters
	for _, r := range name {
		if r < 32 || r == 127 {
			return fmt.Errorf("provider name cannot contain control characters")
		}
	}

	// Check for valid format (alphanumeric, hyphens, underscores)
	if !validProviderNamePattern.MatchString(name) {
		return fmt.Errorf("provider name must be 1-50 characters and contain only letters, numbers, hyphens, and underscores")
	}

	// Check against list of known providers
	if !IsValid(name) {
		return fmt.Errorf("unknown provider '%s', run 'ply setup' to see available providers", name)
	}

	return nil
}
