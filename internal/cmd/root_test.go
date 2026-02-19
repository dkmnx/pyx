package cmd

import (
	"testing"

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
