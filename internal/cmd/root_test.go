package cmd

import (
	"testing"

	"github.com/dkmnx/ply/internal/crypto"
	"github.com/dkmnx/ply/internal/database"
	"github.com/dkmnx/ply/internal/providers"
)

func TestProviderEnvVar(t *testing.T) {
	tests := []struct {
		name          string
		provider      string
		expectedEnv   string
		expectedFound bool
	}{
		{
			name:          "amazon-bedrock",
			provider:      "amazon-bedrock",
			expectedEnv:   "AWS_BEARER_TOKEN_BEDROCK",
			expectedFound: true,
		},
		{
			name:          "anthropic",
			provider:      "anthropic",
			expectedEnv:   "ANTHROPIC_API_KEY",
			expectedFound: true,
		},
		{
			name:          "azure-openai-responses",
			provider:      "azure-openai-responses",
			expectedEnv:   "AZURE_OPENAI_API_KEY",
			expectedFound: true,
		},
		{
			name:          "cerebras",
			provider:      "cerebras",
			expectedEnv:   "CEREBRAS_API_KEY",
			expectedFound: true,
		},
		{
			name:          "github-copilot",
			provider:      "github-copilot",
			expectedEnv:   "GITHUB_TOKEN",
			expectedFound: true,
		},
		{
			name:          "google",
			provider:      "google",
			expectedEnv:   "GEMINI_API_KEY",
			expectedFound: true,
		},
		{
			name:          "google-antigravity",
			provider:      "google-antigravity",
			expectedEnv:   "GEMINI_API_KEY",
			expectedFound: true,
		},
		{
			name:          "google-gemini-cli",
			provider:      "google-gemini-cli",
			expectedEnv:   "GEMINI_API_KEY",
			expectedFound: true,
		},
		{
			name:          "google-vertex",
			provider:      "google-vertex",
			expectedEnv:   "GOOGLE_APPLICATION_CREDENTIALS",
			expectedFound: true,
		},
		{
			name:          "groq",
			provider:      "groq",
			expectedEnv:   "GROQ_API_KEY",
			expectedFound: true,
		},
		{
			name:          "huggingface",
			provider:      "huggingface",
			expectedEnv:   "HF_TOKEN",
			expectedFound: true,
		},
		{
			name:          "kimi-coding",
			provider:      "kimi-coding",
			expectedEnv:   "KIMI_API_KEY",
			expectedFound: true,
		},
		{
			name:          "minimax",
			provider:      "minimax",
			expectedEnv:   "MINIMAX_API_KEY",
			expectedFound: true,
		},
		{
			name:          "minimax-cn",
			provider:      "minimax-cn",
			expectedEnv:   "MINIMAX_CN_API_KEY",
			expectedFound: true,
		},
		{
			name:          "mistral",
			provider:      "mistral",
			expectedEnv:   "MISTRAL_API_KEY",
			expectedFound: true,
		},
		{
			name:          "openai",
			provider:      "openai",
			expectedEnv:   "OPENAI_API_KEY",
			expectedFound: true,
		},
		{
			name:          "openai-codex",
			provider:      "openai-codex",
			expectedEnv:   "OPENAI_API_KEY",
			expectedFound: true,
		},
		{
			name:          "opencode",
			provider:      "opencode",
			expectedEnv:   "OPENCODE_API_KEY",
			expectedFound: true,
		},
		{
			name:          "opencode-zen",
			provider:      "opencode-zen",
			expectedEnv:   "OPENCODE_API_KEY",
			expectedFound: true,
		},
		{
			name:          "openrouter",
			provider:      "openrouter",
			expectedEnv:   "OPENROUTER_API_KEY",
			expectedFound: true,
		},
		{
			name:          "vercel-ai-gateway",
			provider:      "vercel-ai-gateway",
			expectedEnv:   "AI_GATEWAY_API_KEY",
			expectedFound: true,
		},
		{
			name:          "xai",
			provider:      "xai",
			expectedEnv:   "XAI_API_KEY",
			expectedFound: true,
		},
		{
			name:          "zai",
			provider:      "zai",
			expectedEnv:   "ZAI_API_KEY",
			expectedFound: true,
		},
		{
			name:          "unknown provider",
			provider:      "unknown-provider",
			expectedEnv:   "UNKNOWN-PROVIDER_API_KEY",
			expectedFound: true,
		},
		{
			name:          "empty provider",
			provider:      "",
			expectedEnv:   "",
			expectedFound: false,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			envVar, found := providers.EnvVar(tt.provider)
			if found != tt.expectedFound {
				t.Errorf("EnvVar(%q) found = %v, expected %v", tt.provider, found, tt.expectedFound)
			}
			if envVar != tt.expectedEnv {
				t.Errorf("EnvVar(%q) envVar = %q, expected %q", tt.provider, envVar, tt.expectedEnv)
			}
		})
	}
}

func TestDecryptProviderKeys(t *testing.T) {
	masterKey := make([]byte, 32)

	// Create test entries
	cipher1, nonce1, _ := crypto.Encrypt(masterKey, "key1")
	cipher2, nonce2, _ := crypto.Encrypt(masterKey, "key2")

	entries := []database.Entry{
		{Provider: "openai", Cipher: cipher1, Nonce: nonce1},
		{Provider: "anthropic", Cipher: cipher2, Nonce: nonce2},
	}

	keys, err := decryptProviderKeys(masterKey, entries)
	if err != nil {
		t.Fatalf("decryptProviderKeys() error = %v", err)
	}

	if len(keys) != 2 {
		t.Errorf("Expected 2 keys, got %d", len(keys))
	}

	// Verify all keys are decrypted and zeroed when done
	for provider, key := range keys {
		if key == nil {
			t.Errorf("Key for provider %s is nil", provider)
		}
		key.Zero() // Should not panic
	}
}

func TestMapProvidersToEnvVars(t *testing.T) {
	// Create mock SecureStrings
	key1 := crypto.NewSecureString("value1")
	defer key1.Zero()
	key2 := crypto.NewSecureString("value2")
	defer key2.Zero()

	envValues := map[string]*crypto.SecureString{
		"openai":    key1,
		"anthropic": key2,
	}

	result, err := mapProvidersToEnvVars(envValues)
	if err != nil {
		t.Fatalf("mapProvidersToEnvVars() error = %v", err)
	}

	// Verify environment variables are mapped correctly
	if _, ok := result["OPENAI_API_KEY"]; !ok {
		t.Error("OPENAI_API_KEY not found in result")
	}
	if _, ok := result["ANTHROPIC_API_KEY"]; !ok {
		t.Error("ANTHROPIC_API_KEY not found in result")
	}
}

func TestMapProvidersToEnvVars_Conflict(t *testing.T) {
	key1 := crypto.NewSecureString("value1")
	defer key1.Zero()
	key2 := crypto.NewSecureString("value2")
	defer key2.Zero()

	// Both map to same env var (openai and openai-codex both use OPENAI_API_KEY)
	envValues := map[string]*crypto.SecureString{
		"openai":       key1,
		"openai-codex": key2,
	}

	_, err := mapProvidersToEnvVars(envValues)
	if err == nil {
		t.Error("Expected error for conflicting keys")
	}
}

func TestBuildEnvSlice(t *testing.T) {
	key1 := crypto.NewSecureString("value1")
	defer key1.Zero()
	key2 := crypto.NewSecureString("value2")
	defer key2.Zero()

	envValues := map[string]*crypto.SecureString{
		"OPENAI_API_KEY":    key1,
		"ANTHROPIC_API_KEY": key2,
	}

	result := buildEnvSlice(envValues)

	// Verify our custom env vars are added
	foundOpenAI := false
	foundAnthropic := false
	for _, env := range result {
		if len(env) > len("OPENAI_API_KEY=") && env[:len("OPENAI_API_KEY=")] == "OPENAI_API_KEY=" {
			foundOpenAI = true
		}
		if len(env) > len("ANTHROPIC_API_KEY=") && env[:len("ANTHROPIC_API_KEY=")] == "ANTHROPIC_API_KEY=" {
			foundAnthropic = true
		}
	}

	if !foundOpenAI {
		t.Error("OPENAI_API_KEY not found in result")
	}
	if !foundAnthropic {
		t.Error("ANTHROPIC_API_KEY not found in result")
	}
}
