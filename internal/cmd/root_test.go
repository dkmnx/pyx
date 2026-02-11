package cmd

import (
	"testing"
)

func TestProviderEnvVar(t *testing.T) {
	tests := []struct {
		name          string
		provider      string
		expectedEnv   string
		expectedFound bool
	}{
		{
			name:          "anthropic",
			provider:      "anthropic",
			expectedEnv:   "ANTHROPIC_API_KEY",
			expectedFound: true,
		},
		{
			name:          "openai",
			provider:      "openai",
			expectedEnv:   "OPENAI_API_KEY",
			expectedFound: true,
		},
		{
			name:          "google",
			provider:      "google",
			expectedEnv:   "GEMINI_API_KEY",
			expectedFound: true,
		},
		{
			name:          "gemini",
			provider:      "gemini",
			expectedEnv:   "GEMINI_API_KEY",
			expectedFound: true,
		},
		{
			name:          "azure-openai-responses",
			provider:      "azure-openai-responses",
			expectedEnv:   "AZURE_OPENAI_API_KEY",
			expectedFound: true,
		},
		{
			name:          "mistral",
			provider:      "mistral",
			expectedEnv:   "MISTRAL_API_KEY",
			expectedFound: true,
		},
		{
			name:          "groq",
			provider:      "groq",
			expectedEnv:   "GROQ_API_KEY",
			expectedFound: true,
		},
		{
			name:          "cerebras",
			provider:      "cerebras",
			expectedEnv:   "CEREBRAS_API_KEY",
			expectedFound: true,
		},
		{
			name:          "xai",
			provider:      "xai",
			expectedEnv:   "XAI_API_KEY",
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
			name:          "zai",
			provider:      "zai",
			expectedEnv:   "ZAI_API_KEY",
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
			name:          "unknown provider",
			provider:      "unknown-provider",
			expectedEnv:   "",
			expectedFound: false,
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
			envVar, found := providerEnvVar(tt.provider)
			if found != tt.expectedFound {
				t.Errorf("providerEnvVar(%q) found = %v, expected %v", tt.provider, found, tt.expectedFound)
			}
			if envVar != tt.expectedEnv {
				t.Errorf("providerEnvVar(%q) envVar = %q, expected %q", tt.provider, envVar, tt.expectedEnv)
			}
		})
	}
}
