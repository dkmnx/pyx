package providers

import (
	"context"
	"fmt"
	"os"
	"path/filepath"
	"regexp"
	"strings"

	"github.com/dkmnx/ply/internal/models"
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

// envVarMapping provides the environment variable name for known providers.
// This is needed because the env var is not in the fetched models file.
//
// Maintenance: When new providers are added to pi-mono, they should be added here.
// However, the deriveEnvVar() function provides a fallback for unknown providers,
// so manual updates are not strictly required for basic functionality.
//
// To add a new provider:
// 1. Add entry: "provider-name": "PROVIDER_API_KEY",
// 2. Run tests to ensure mapping is correct
// 3. Update docs if needed
var envVarMapping = map[string]string{
	"amazon-bedrock":         "AWS_BEARER_TOKEN_BEDROCK",
	"anthropic":              "ANTHROPIC_API_KEY",
	"azure-openai-responses": "AZURE_OPENAI_API_KEY",
	"cerebras":               "CEREBRAS_API_KEY",
	"github-copilot":         "GITHUB_TOKEN",
	"google":                 "GEMINI_API_KEY",
	"google-antigravity":     "GEMINI_API_KEY",
	"google-gemini-cli":      "GEMINI_API_KEY",
	"google-vertex":          "GOOGLE_APPLICATION_CREDENTIALS",
	"groq":                   "GROQ_API_KEY",
	"huggingface":            "HF_TOKEN",
	"kimi-coding":            "KIMI_API_KEY",
	"minimax":                "MINIMAX_API_KEY",
	"minimax-cn":             "MINIMAX_CN_API_KEY",
	"mistral":                "MISTRAL_API_KEY",
	"openai":                 "OPENAI_API_KEY",
	"openai-codex":           "OPENAI_API_KEY",
	"opencode":               "OPENCODE_API_KEY",
	"opencode-zen":           "OPENCODE_API_KEY",
	"openrouter":             "OPENROUTER_API_KEY",
	"qwen":                   "QWEN_API_KEY",
	"vercel-ai-gateway":      "AI_GATEWAY_API_KEY",
	"xai":                    "XAI_API_KEY",
	"zai":                    "ZAI_API_KEY",
}

// Names returns a list of all supported provider names fetched from models cache.
// If cache is unavailable or stale, returns an empty list.
func Names(ctx context.Context) []string {
	all, err := models.GetModels(ctx)
	if err != nil {
		return []string{}
	}
	names := make([]string, 0, len(all))
	for provider := range all {
		names = append(names, provider)
	}

	// Add qwen if extension exists
	if hasQwenExtension() {
		names = append(names, "qwen")
	}

	return names
}

// EnvVar returns the environment variable name required for the given provider.
// Returns the env var name and true if the provider is found.
// Returns empty string and false if the provider is not recognized.
func EnvVar(name string) (string, bool) {
	if name == "" {
		return "", false
	}

	envVar, ok := envVarMapping[name]
	if ok {
		return envVar, true
	}

	// Try to derive from provider name
	derived, ok := deriveEnvVar(name)
	if ok {
		// Add to mapping for next time
		envVarMapping[name] = derived
		return derived, true
	}

	return "", false
}

// providerEnvVarPattern maps provider name patterns to environment variable names.
// Keys are prefixes or special patterns, values are the corresponding env var.
var providerEnvVarPattern = map[string]string{
	"amazon":      "AWS_BEARER_TOKEN_BEDROCK",
	"aws":         "AWS_BEARER_TOKEN_BEDROCK",
	"azure":       "AZURE_OPENAI_API_KEY",
	"anthropic":   "ANTHROPIC_API_KEY",
	"openai":      "OPENAI_API_KEY",
	"github":      "GITHUB_TOKEN",
	"huggingface": "HF_TOKEN",
	"hugging":     "HF_TOKEN",
	"cohere":      "COHERE_API_KEY",
	"deepseek":    "DEEPSEEK_API_KEY",
	"meta":        "META_API_KEY",
	"nvidia":      "NVIDIA_API_KEY",
	"moonshot":    "MOONSHOT_API_KEY",
	"qwen":        "QWEN_API_KEY",
	"writer":      "WRITER_API_KEY",
	"mistral":     "MISTRAL_API_KEY",
	"groq":        "GROQ_API_KEY",
	"openrouter":  "OPENROUTER_API_KEY",
	"cerebras":    "CEREBRAS_API_KEY",
	"kimi":        "KIMI_API_KEY",
	"opencode":    "OPENCODE_API_KEY",
	"xai":         "XAI_API_KEY",
	"zai":         "ZAI_API_KEY",
}

// deriveEnvVar attempts to derive the environment variable name from the provider name.
// This handles cases where a new provider was added to pi-mono but not yet to our mapping.
// The function uses common naming patterns to make an educated guess.
//
// Priority order:
// 1. Special cases (google-vertex, minimax-cn, vercel)
// 2. Known provider prefixes from providerEnvVarPattern
// 3. Pattern-based derivation (-ai suffix)
// 4. Generic pattern: PROVIDER_NAME_API_KEY
func deriveEnvVar(name string) (string, bool) {
	// Special cases that need custom handling
	if strings.HasPrefix(name, "google") && strings.Contains(name, "vertex") {
		return "GOOGLE_APPLICATION_CREDENTIALS", true
	}
	if strings.HasPrefix(name, "google") {
		return "GEMINI_API_KEY", true
	}
	if name == "minimax-cn" || strings.HasSuffix(name, "-cn") {
		return "MINIMAX_CN_API_KEY", true
	}
	if strings.HasPrefix(name, "minimax") {
		return "MINIMAX_API_KEY", true
	}
	if strings.HasPrefix(name, "vercel") {
		return "AI_GATEWAY_API_KEY", true
	}

	// Check known provider patterns
	for prefix, envVar := range providerEnvVarPattern {
		if strings.HasPrefix(name, prefix) {
			return envVar, true
		}
	}

	// Pattern-based derivation for -ai or _ai suffix
	if strings.HasSuffix(name, "-ai") || strings.HasSuffix(name, "_ai") {
		base := strings.TrimSuffix(name, "-ai")
		base = strings.TrimSuffix(base, "_ai")
		base = strings.ToUpper(strings.ReplaceAll(base, "-", "_"))
		return base + "_API_KEY", true
	}

	// Default fallback: convert provider name to uppercase and append _API_KEY
	nameUpper := strings.ToUpper(strings.ReplaceAll(name, "-", "_"))
	return nameUpper + "_API_KEY", true
}

// hasQwenExtension checks if the qwen-coding-plan-provider extension exists.
func hasQwenExtension() bool {
	homeDir := os.Getenv("HOME")
	if homeDir == "" {
		homeDir = os.Getenv("USERPROFILE") // Windows fallback
	}
	if homeDir == "" {
		return false
	}
	extPath := filepath.Join(homeDir, ".pi", "agent", "extensions", "qwen-coding-plan-provider")
	info, err := os.Stat(extPath)
	return err == nil && info.IsDir()
}

// IsValid returns true if the given provider name is recognized.
// This checks against the providers fetched from models cache.
// Deprecated: Use Validate() instead which provides better error messages.
func IsValid(name string) bool {
	all, err := models.GetModels(context.Background())
	if err != nil {
		return false
	}
	_, exists := all[name]

	// Also check for qwen extension
	if name == "qwen" && hasQwenExtension() {
		return true
	}

	return exists
}

// IsValidWithContext returns true if the given provider name is recognized.
// This checks against the providers fetched from models cache using the provided context.
func IsValidWithContext(ctx context.Context, name string) bool {
	all, err := models.GetModels(ctx)
	if err != nil {
		return false
	}
	_, exists := all[name]

	// Also check for qwen extension
	if name == "qwen" && hasQwenExtension() {
		return true
	}

	return exists
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

	// Check against list of known providers from cache
	if !IsValid(name) {
		// Check if env var is known (might be a new provider not yet in cache)
		if _, ok := EnvVar(name); !ok {
			return fmt.Errorf("unknown provider '%s'. Run 'ply models update' to refresh the provider list, or verify the provider name", name)
		}
	}

	return nil
}

// GetAll returns all providers with their environment variables from the models cache.
func GetAll(ctx context.Context) ([]Provider, error) {
	allModels, err := models.GetModels(ctx)
	if err != nil {
		return nil, fmt.Errorf("failed to get models: %w", err)
	}

	providers := make([]Provider, 0, len(allModels))
	for name := range allModels {
		envVar, ok := EnvVar(name)
		if !ok {
			// Skip providers without env var mapping
			continue
		}
		providers = append(providers, Provider{Name: name, EnvVar: envVar})
	}

	// Add qwen if extension exists
	if hasQwenExtension() {
		envVar, _ := EnvVar("qwen") // We just added it to mapping, so this will succeed
		providers = append(providers, Provider{Name: "qwen", EnvVar: envVar})
	}

	return providers, nil
}
