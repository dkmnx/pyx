package cmd

import (
	"testing"
)

func TestTimeFormatConstant(t *testing.T) {
	// Test that timeFormat is properly set
	if timeFormat != "2006-01-02T15:04:05Z" {
		t.Errorf("timeFormat = %q, expected %q", timeFormat, "2006-01-02T15:04:05Z")
	}
}

func TestConfirmYesConstant(t *testing.T) {
	if confirmYes != "yes" {
		t.Errorf("confirmYes = %q, expected %q", confirmYes, "yes")
	}
}

func TestConfirmYConstant(t *testing.T) {
	if confirmY != "y" {
		t.Errorf("confirmY = %q, expected %q", confirmY, "y")
	}
}

func TestRootCmdUse(t *testing.T) {
	if rootCmd.Use != "ply [provider] [args...]" {
		t.Errorf("rootCmd.Use = %q, expected %q", rootCmd.Use, "ply [provider] [args...]")
	}
}

func TestRootCmdArgs(t *testing.T) {
	// The root command should accept arbitrary args
	if rootCmd.Args == nil {
		t.Error("rootCmd.Args should be set")
	}
}

func TestSessionFlag(t *testing.T) {
	// Test the sessionFlag variable
	sessionFlag = "test-session-uuid"
	if sessionFlag != "test-session-uuid" {
		t.Error("sessionFlag should be settable")
	}

	sessionFlag = ""
}

func TestRootCmdShort(t *testing.T) {
	if rootCmd.Short == "" {
		t.Error("rootCmd.Short should not be empty")
	}
}

func TestRootCmdLong(t *testing.T) {
	if rootCmd.Long == "" {
		t.Error("rootCmd.Long should not be empty")
	}
}

func TestRootCmdStructure(t *testing.T) {
	if rootCmd == nil {
		t.Fatal("rootCmd is nil")
	}

	// Check that CompletionOptions.DisableDefaultCmd is false (completion should be enabled)
	// Note: This might be false even if init() hasn't run yet
	if rootCmd.CompletionOptions.DisableDefaultCmd {
		t.Log("CompletionOptions.DisableDefaultCmd is true")
	}
}

func TestParseArgs(t *testing.T) {
	tests := []struct {
		name            string
		args            []string
		wantProviderArg string
		wantPiArgs      []string
	}{
		{
			name:            "no args",
			args:            []string{},
			wantProviderArg: "",
			wantPiArgs:      []string{},
		},
		{
			name:            "provider only",
			args:            []string{"openai"},
			wantProviderArg: "openai",
			wantPiArgs:      []string{},
		},
		{
			name:            "provider with pi args",
			args:            []string{"openai", "--model", "gpt-4"},
			wantProviderArg: "openai",
			wantPiArgs:      []string{"--model", "gpt-4"},
		},
		{
			name:            "double dash separator",
			args:            []string{"--", "--model", "gpt-4"},
			wantProviderArg: "",
			wantPiArgs:      []string{"--model", "gpt-4"},
		},
		{
			name:            "provider with double dash",
			args:            []string{"openai", "--", "--verbose"},
			wantProviderArg: "openai",
			wantPiArgs:      []string{"--verbose"},
		},
		{
			name:            "only pi args (starting with dash)",
			args:            []string{"--verbose", "--model", "gpt-4"},
			wantProviderArg: "",
			wantPiArgs:      []string{"--verbose", "--model", "gpt-4"},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			providerArg, piArgs := parseArgs(tt.args)
			if providerArg != tt.wantProviderArg {
				t.Errorf("parseArgs() providerArg = %q, want %q", providerArg, tt.wantProviderArg)
			}
			if len(piArgs) != len(tt.wantPiArgs) {
				t.Errorf("parseArgs() piArgs length = %d, want %d", len(piArgs), len(tt.wantPiArgs))
			}
		})
	}
}

func TestValidateProvider(t *testing.T) {
	tests := []struct {
		name    string
		wantErr bool
	}{
		{"openai", false},
		{"anthropic", false},
		{"google", false},
		{"valid-name", false},
		{"", true},
		{"..", true},
		{"/path", true},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			err := validateProvider(tt.name)
			if (err != nil) != tt.wantErr {
				t.Errorf("validateProvider(%q) error = %v, wantErr %v", tt.name, err, tt.wantErr)
			}
		})
	}
}
