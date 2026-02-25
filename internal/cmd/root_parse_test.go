package cmd

import (
	"strings"
	"testing"
)

func TestParseArgs(t *testing.T) {
	tests := []struct {
		name               string
		args               []string
		expectedProvider   string
		expectedPiArgs     []string
		expectedSkipFilter bool
	}{
		{
			name:               "no args",
			args:               []string{},
			expectedProvider:   "",
			expectedPiArgs:     nil,
			expectedSkipFilter: false,
		},
		{
			name:               "single provider",
			args:               []string{"openai"},
			expectedProvider:   "openai",
			expectedPiArgs:     nil,
			expectedSkipFilter: false,
		},
		{
			name:               "provider with args",
			args:               []string{"openai", "Hello", "world"},
			expectedProvider:   "openai",
			expectedPiArgs:     []string{"Hello", "world"},
			expectedSkipFilter: false,
		},
		{
			name:               "flag args only",
			args:               []string{"--version"},
			expectedProvider:   "",
			expectedPiArgs:     []string{"--version"},
			expectedSkipFilter: false,
		},
		{
			name:               "double dash separator",
			args:               []string{"openai", "--", "-m", "gpt-4"},
			expectedProvider:   "openai",
			expectedPiArgs:     []string{"-m", "gpt-4"},
			expectedSkipFilter: true,
		},
		{
			name:               "double dash at start",
			args:               []string{"--", "--version"},
			expectedProvider:   "",
			expectedPiArgs:     []string{"--version"},
			expectedSkipFilter: true,
		},
		{
			name:               "multiple flags",
			args:               []string{"--verbose", "--model", "gpt-4"},
			expectedProvider:   "",
			expectedPiArgs:     []string{"--verbose", "--model", "gpt-4"},
			expectedSkipFilter: false,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			provider, piArgs, skipFilter := parseArgs(tt.args)

			if provider != tt.expectedProvider {
				t.Errorf("parseArgs(%v) provider = %q, expected %q", tt.args, provider, tt.expectedProvider)
			}

			if len(piArgs) != len(tt.expectedPiArgs) {
				t.Errorf("parseArgs(%v) piArgs = %v, expected %v", tt.args, piArgs, tt.expectedPiArgs)
			} else {
				for i := range piArgs {
					if piArgs[i] != tt.expectedPiArgs[i] {
						t.Errorf("parseArgs(%v) piArgs[%d] = %q, expected %q", tt.args, i, piArgs[i], tt.expectedPiArgs[i])
					}
				}
			}

			if skipFilter != tt.expectedSkipFilter {
				t.Errorf("parseArgs(%v) skipFilter = %v, expected %v", tt.args, skipFilter, tt.expectedSkipFilter)
			}
		})
	}
}

func TestZeroMasterKey(t *testing.T) {
	// Create a test master key
	masterKey := []byte("this-is-a-32-byte-test-key-here!")

	// Verify length is correct (should be 32 bytes)
	if len(masterKey) != 32 {
		t.Fatalf("Test setup error: expected 32 bytes, got %d", len(masterKey))
	}

	// Zero the master key
	zeroMasterKey(masterKey)

	// Check that all bytes are zero
	for i, b := range masterKey {
		if b != 0 {
			t.Errorf("zeroMasterKey() masterKey[%d] = %d, expected 0", i, b)
		}
	}
}

func TestZeroMasterKey_Empty(t *testing.T) {
	// Test with empty slice - should not panic
	masterKey := []byte{}
	zeroMasterKey(masterKey)

	if len(masterKey) != 0 {
		t.Errorf("zeroMasterKey() modified empty slice length from 0 to %d", len(masterKey))
	}
}

func TestValidateProvider(t *testing.T) {
	tests := []struct {
		name        string
		provider    string
		wantErr     bool
		errContains string
	}{
		{"valid provider format", "openai", false, ""},
		{"empty provider", "", true, "cannot be empty"},
		{"path traversal", "../etc", true, "path traversal"},
		{"invalid characters", "provider@bad", true, "must be 1-50 characters"},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			err := validateProvider(tt.provider)
			if (err != nil) != tt.wantErr {
				t.Errorf("validateProvider() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			if err != nil && tt.errContains != "" {
				if !strings.Contains(err.Error(), tt.errContains) {
					t.Errorf("validateProvider() error = %v, should contain %q", err, tt.errContains)
				}
			}
		})
	}
}
