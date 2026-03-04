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
// When new providers are added to pi-mono, they should be added here.
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

// deriveEnvVar attempts to derive the environment variable name from the provider name.
// This handles cases where a new provider was added to pi-mono but not yet to our mapping.
func deriveEnvVar(name string) (string, bool) {
	nameUpper := strings.ToUpper(name)

	// Common patterns
	switch {
	case strings.HasPrefix(name, "amazon"):
		return "AWS_BEARER_TOKEN_BEDROCK", true
	case strings.HasPrefix(name, "anthropic"):
		return "ANTHROPIC_API_KEY", true
	case strings.HasPrefix(name, "azure"):
		return "AZURE_OPENAI_API_KEY", true
	case strings.HasPrefix(name, "cohere"):
		return "COHERE_API_KEY", true
	case strings.HasPrefix(name, "deepseek"):
		return "DEEPSEEK_API_KEY", true
	case strings.HasPrefix(name, "github"):
		return "GITHUB_TOKEN", true
	case strings.HasPrefix(name, "google"):
		if strings.Contains(name, "vertex") {
			return "GOOGLE_APPLICATION_CREDENTIALS", true
		}
		return "GEMINI_API_KEY", true
	case strings.HasPrefix(name, "meta"):
		return "META_API_KEY", true
	case strings.HasPrefix(name, "nvidia"):
		return "NVIDIA_API_KEY", true
	case strings.HasPrefix(name, "moonshot"):
		return "MOONSHOT_API_KEY", true
	case strings.HasPrefix(name, "qwen"):
		return "QWEN_API_KEY", true
	case strings.HasPrefix(name, "writer"):
		return "WRITER_API_KEY", true
	case strings.HasSuffix(name, "-ai"):
		base := strings.TrimSuffix(name, "-ai")
		return strings.ToUpper(base) + "_API_KEY", true
	default:
		return nameUpper + "_API_KEY", true
	}
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
