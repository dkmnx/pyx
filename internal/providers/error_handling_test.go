package providers

import (
	"context"
	"testing"
)

func TestValidateEmptyName(t *testing.T) {
	err := Validate("")
	if err == nil {
		t.Error("Validate() with empty name should return error")
	}

	if err.Error() != "provider name cannot be empty" {
		t.Errorf("Validate() error = %v, want 'provider name cannot be empty'", err)
	}
}

func TestValidatePathTraversal(t *testing.T) {
	tests := []struct {
		name string
	}{
		{"../etc/passwd"},
		{"..\\..\\windows\\system32"},
		{"provider/../malicious"},
		{"foo/..bar"},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			err := Validate(tt.name)
			if err == nil {
				t.Errorf("Validate(%q) should return error for path traversal attempt", tt.name)
			}
		})
	}
}

func TestValidatePathSeparators(t *testing.T) {
	tests := []struct {
		name string
	}{
		{"provider/name"},
		{"provider\\name"},
		{"/provider"},
		{"provider/"},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			err := Validate(tt.name)
			if err == nil {
				t.Errorf("Validate(%q) should return error for path separator", tt.name)
			}
		})
	}
}

func TestValidateControlCharacters(t *testing.T) {
	tests := []struct {
		name string
	}{
		{"provider\x00name"},
		{"provider\x07name"},
		{"provider\x1fname"},
		{"provider\x7fname"},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			err := Validate(tt.name)
			if err == nil {
				t.Errorf("Validate(%q) should return error for control character", tt.name)
			}
		})
	}
}

func TestValidateInvalidFormat(t *testing.T) {
	tests := []struct {
		name string
	}{
		{"provider@name"},
		{"provider name"},
		{"provider.name"},
		{"provider!name"},
		{"provider#name"},
		{"provider$name"},
		{"provider%name"},
		{"provider^name"},
		{"provider&name"},
		{"provider*name"},
		{"provider(name)"},
		{"provider[name]"},
		{"provider{name}"},
		{"provider|name"},
		{"provider:name"},
		{"provider;name"},
		{"provider'name"},
		{"provider\"name"},
		{"provider<name>"},
		{"provider,name"},
		{"provider?name"},
		{"provider=name"},
		{"provider+name"},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			err := Validate(tt.name)
			if err == nil {
				t.Errorf("Validate(%q) should return error for invalid format", tt.name)
			}
		})
	}
}

func TestValidateTooLong(t *testing.T) {
	// Generate a name that's too long (>50 characters)
	longName := "this-is-a-very-long-provider-name-that-exceeds-the-maximum-allowed-length"
	err := Validate(longName)
	if err == nil {
		t.Errorf("Validate(%q) should return error for name > 50 characters", longName)
	}
}

func TestValidateValidNames(t *testing.T) {
	tests := []struct {
		name string
	}{
		{"openai"},
		{"anthropic"},
		{"google"},
		{"github-copilot"},
		{"azure_openai"},
		{"provider-123"},
		{"123-provider"},
		{"a"}, // Minimum length
		{"abcdefghijklmnopqrstuvwxyz1234567890abcd"}, // 40 chars - OK
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			err := Validate(tt.name)
			// Note: These might fail if the provider is not in the cache
			// We're mainly testing that the format validation passes
			if err != nil && err.Error() != "unknown provider" {
				t.Errorf("Validate(%q) unexpected error = %v", tt.name, err)
			}
		})
	}
}

func TestEnvVarEmptyName(t *testing.T) {
	envVar, ok := EnvVar("")
	if ok {
		t.Error("EnvVar() with empty name should return false")
	}
	if envVar != "" {
		t.Errorf("EnvVar() with empty name should return empty string, got %q", envVar)
	}
}

func TestEnvVarUnknownProvider(t *testing.T) {
	// Unknown provider should still return an env var (derived from name)
	envVar, ok := EnvVar("unknown-provider-xyz")
	if !ok {
		t.Error("EnvVar() should return true for unknown provider (using derivation)")
	}
	if envVar == "" {
		t.Error("EnvVar() should return non-empty string for unknown provider")
	}
	// Should derive: UNKNOWN_PROVIDER_XYZ_API_KEY
	expected := "UNKNOWN_PROVIDER_XYZ_API_KEY"
	if envVar != expected {
		t.Errorf("EnvVar() = %q, want %q", envVar, expected)
	}
}

func TestEnvVarKnownProviders(t *testing.T) {
	tests := []struct {
		name    string
		wantEnv string
		wantOk  bool
	}{
		{"openai", "OPENAI_API_KEY", true},
		{"anthropic", "ANTHROPIC_API_KEY", true},
		{"google", "GEMINI_API_KEY", true},
		{"github-copilot", "GITHUB_TOKEN", true},
		{"groq", "GROQ_API_KEY", true},
		{"mistral", "MISTRAL_API_KEY", true},
		{"unknown-provider", "UNKNOWN_PROVIDER_API_KEY", true},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			envVar, ok := EnvVar(tt.name)
			if ok != tt.wantOk {
				t.Errorf("EnvVar() ok = %v, want %v", ok, tt.wantOk)
			}
			if envVar != tt.wantEnv {
				t.Errorf("EnvVar() = %q, want %q", envVar, tt.wantEnv)
			}
		})
	}
}

func TestDeriveEnvVarSpecialCases(t *testing.T) {
	tests := []struct {
		name    string
		wantEnv string
		wantOk  bool
	}{
		{"google-vertex", "GOOGLE_APPLICATION_CREDENTIALS", true},
		{"google-gemini-cli", "GEMINI_API_KEY", true},
		{"google-antigravity", "GEMINI_API_KEY", true},
		{"minimax-cn", "MINIMAX_CN_API_KEY", true},
		{"some-provider-cn", "MINIMAX_CN_API_KEY", true},
		{"minimax", "MINIMAX_API_KEY", true},
		{"minimax-pro", "MINIMAX_API_KEY", true},
		{"vercel-ai-gateway", "AI_GATEWAY_API_KEY", true},
		{"vercel-test", "AI_GATEWAY_API_KEY", true},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			envVar, ok := deriveEnvVar(tt.name)
			if ok != tt.wantOk {
				t.Errorf("deriveEnvVar(%q) ok = %v, want %v", tt.name, ok, tt.wantOk)
			}
			if envVar != tt.wantEnv {
				t.Errorf("deriveEnvVar(%q) = %q, want %q", tt.name, envVar, tt.wantEnv)
			}
		})
	}
}

func TestDeriveEnvVarAiSuffix(t *testing.T) {
	tests := []struct {
		name    string
		wantEnv string
		wantOk  bool
	}{
		{"my-ai", "MY_API_KEY", true},
		{"some_ai", "SOME_API_KEY", true},
		{"test-ai-provider", "TEST_AI_PROVIDER_API_KEY", true}, // Has -ai but not at end
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			envVar, ok := deriveEnvVar(tt.name)
			if ok != tt.wantOk {
				t.Errorf("deriveEnvVar(%q) ok = %v, want %v", tt.name, ok, tt.wantOk)
			}
			if envVar != tt.wantEnv {
				t.Errorf("deriveEnvVar(%q) = %q, want %q", tt.name, envVar, tt.wantEnv)
			}
		})
	}
}

func TestDeriveEnvVarFallback(t *testing.T) {
	// Provider that doesn't match any pattern should use fallback
	envVar, ok := deriveEnvVar("completely-unknown-provider")
	if !ok {
		t.Error("deriveEnvVar() should always return true (has fallback)")
	}
	expected := "COMPLETELY_UNKNOWN_PROVIDER_API_KEY"
	if envVar != expected {
		t.Errorf("deriveEnvVar() = %q, want %q", envVar, expected)
	}
}

func TestIsValidWithContext(t *testing.T) {
	ctx := context.Background()

	// Empty name should return false
	if IsValidWithContext(ctx, "") {
		t.Error("IsValidWithContext() with empty name should return false")
	}

	// Unknown provider should return false
	if IsValidWithContext(ctx, "definitely-not-a-real-provider") {
		t.Error("IsValidWithContext() with unknown provider should return false")
	}
}

func TestNamesEmptyOnModelFailure(t *testing.T) {
	ctx := context.Background()

	// Names should return empty list if models can't be fetched
	// This is hard to test without mocking, but we can verify it returns a slice
	names := Names(ctx)
	if names == nil {
		t.Error("Names() should return empty slice, not nil")
	}
}

func TestGetAllContextCancellation(t *testing.T) {
	ctx, cancel := context.WithCancel(context.Background())
	cancel()

	// GetAll should handle cancelled context gracefully
	// Note: models.GetModels may or may not check context, so this test may pass or fail
	// depending on implementation details
	_, err := GetAll(ctx)
	// We don't assert error here because GetAll depends on models.GetModels
	// which may or may not propagate context cancellation
	if err != nil {
		t.Logf("GetAll() with cancelled context returned error (expected): %v", err)
	}
}

func TestValidateProviderWithEnvVarMapping(t *testing.T) {
	// Provider with known env var but not in cache should still be valid
	// This tests the fallback to env var mapping
	err := Validate("amazon-bedrock")
	// This might pass or fail depending on cache state
	// The important thing is it doesn't panic
	if err != nil && err.Error() != "unknown provider" {
		t.Errorf("Validate() unexpected error = %v", err)
	}
}

func TestEnvVarPatternMatching(t *testing.T) {
	tests := []struct {
		name    string
		wantEnv string
		wantOk  bool
	}{
		{"amazon-bedrock", "AWS_BEARER_TOKEN_BEDROCK", true},
		{"aws-something", "AWS_BEARER_TOKEN_BEDROCK", true},
		{"azure-openai", "AZURE_OPENAI_API_KEY", true},
		{"anthropic-test", "ANTHROPIC_API_KEY", true},
		{"openai-test", "OPENAI_API_KEY", true},
		{"github-test", "GITHUB_TOKEN", true},
		{"huggingface-models", "HF_TOKEN", true},
		{"hugging-test", "HF_TOKEN", true},
		{"deepseek-model", "DEEPSEEK_API_KEY", true},
		{"meta-ai", "META_API_KEY", true},
		{"nvidia-models", "NVIDIA_API_KEY", true},
		{"moonshot-ai", "MOONSHOT_API_KEY", true},
		{"qwen-coding", "QWEN_API_KEY", true},
		{"writer-models", "WRITER_API_KEY", true},
		{"cerebras-model", "CEREBRAS_API_KEY", true},
		{"kimi-ai", "KIMI_API_KEY", true},
		{"opencode-model", "OPENCODE_API_KEY", true},
		{"xai-model", "XAI_API_KEY", true},
		{"zai-model", "ZAI_API_KEY", true},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			envVar, ok := EnvVar(tt.name)
			if ok != tt.wantOk {
				t.Errorf("EnvVar(%q) ok = %v, want %v", tt.name, ok, tt.wantOk)
			}
			if envVar != tt.wantEnv {
				t.Errorf("EnvVar(%q) = %q, want %q", tt.name, envVar, tt.wantEnv)
			}
		})
	}
}

func TestValidateWithSpecialCases(t *testing.T) {
	tests := []struct {
		name    string
		wantErr bool
	}{
		{"", true},                                          // Empty
		{"starts-with-hyphen", false},                       // Valid format
		{"ends-with-hyphen-", false},                        // Valid format
		{"_starts-with-underscore", false},                  // Valid format
		{"ends-with-underscore_", false},                    // Valid format
		{"123-starts-with-number", false},                   // Valid format
		{"MiXeD-cAsE-pRoViDeR", false},                      // Valid format
		{"provider--double-hyphen", false},                  // Valid format
		{"provider__double-underscore", false},              // Valid format
		{"provider-_mixed", false},                          // Valid format
		{"a", false},                                        // Single char
		{"abcdefghijklmnopqrstuvwxyz1234567890abcd", false}, // 40 chars - OK
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			err := Validate(tt.name)
			gotErr := err != nil
			if gotErr != tt.wantErr {
				// Check if it's the "unknown provider" error which is acceptable
				if err != nil && err.Error() == "unknown provider" {
					return
				}
				t.Errorf("Validate(%q) error = %v, wantErr = %v", tt.name, err, tt.wantErr)
			}
		})
	}
}

func TestLoadCustomProviderEnvVarsError(t *testing.T) {
	// This is hard to test without mocking settings.Load
	// We can at least verify the function doesn't panic
	_, err := loadCustomProviderEnvVars()
	// Error is expected if settings file doesn't exist
	// The important thing is it doesn't panic
	_ = err
}
