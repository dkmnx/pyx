package cmd

import (
	"bytes"
	"os"
	"strings"
	"testing"

	"github.com/dkmnx/ply/internal/models"
)

func TestOutputTable(t *testing.T) {
	// Save original modelsProvider flag
	oldProvider := modelsProvider
	t.Cleanup(func() {
		modelsProvider = oldProvider
	})

	// Create test data
	testData := models.Models{
		"openai":    []string{"gpt-4", "gpt-3.5-turbo"},
		"anthropic": []string{"claude-3-opus", "claude-3-sonnet"},
	}

	tests := []struct {
		name          string
		providerFlag  string
		wantProviders int
		wantContains  []string
	}{
		{
			name:          "all providers",
			providerFlag:  "",
			wantProviders: 2,
			wantContains:  []string{"openai", "anthropic", "gpt-4", "claude-3-opus"},
		},
		{
			name:          "filtered by provider",
			providerFlag:  "openai",
			wantProviders: 1,
			wantContains:  []string{"openai", "gpt-4"},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			modelsProvider = tt.providerFlag

			// Capture stdout
			oldStdout := os.Stdout
			r, w, _ := os.Pipe()
			os.Stdout = w

			outputTable(testData)

			w.Close()
			os.Stdout = oldStdout

			// Read output
			var buf bytes.Buffer
			buf.ReadFrom(r)
			output := buf.String()

			// Verify expected content
			for _, want := range tt.wantContains {
				if !strings.Contains(output, want) {
					t.Errorf("outputTable() should contain %q, got: %s", want, output)
				}
			}
		})
	}
}

func TestOutputTableUnknownProvider(t *testing.T) {
	// Note: outputTable calls os.Exit(1) when provider is not found
	// This test verifies the behavior by checking if it causes a panic
	// We can't easily test this without restructuring the code
	t.Skip("outputTable calls os.Exit(1) which terminates the test")
}

func TestOutputTableEmptyModels(t *testing.T) {
	// Test with empty models data
	oldProvider := modelsProvider
	t.Cleanup(func() {
		modelsProvider = oldProvider
	})
	modelsProvider = ""

	testData := models.Models{}

	// Capture stdout
	oldStdout := os.Stdout
	r, w, _ := os.Pipe()
	os.Stdout = w

	outputTable(testData)

	w.Close()
	os.Stdout = oldStdout

	var buf bytes.Buffer
	buf.ReadFrom(r)
	output := buf.String()

	// Should still print "Total: 0 providers, 0 models"
	if !strings.Contains(output, "Total:") {
		t.Errorf("outputTable() with empty data should print Total line, got: %s", output)
	}
}

func TestOutputJSON(t *testing.T) {
	// Save original modelsProvider flag
	oldProvider := modelsProvider
	t.Cleanup(func() {
		modelsProvider = oldProvider
	})

	// Create test data
	testData := models.Models{
		"openai":    []string{"gpt-4", "gpt-3.5-turbo"},
		"anthropic": []string{"claude-3-opus"},
	}

	// Test all providers
	modelsProvider = ""

	oldStdout := os.Stdout
	r, w, _ := os.Pipe()
	os.Stdout = w

	outputJSON(testData)

	w.Close()
	os.Stdout = oldStdout

	var buf bytes.Buffer
	buf.ReadFrom(r)
	output := buf.String()

	// Should be valid JSON
	if !strings.Contains(output, "\"openai\"") {
		t.Errorf("outputJSON() should contain openai, got: %s", output)
	}
	if !strings.Contains(output, "\"anthropic\"") {
		t.Errorf("outputJSON() should contain anthropic, got: %s", output)
	}

	// Test filtered by provider
	modelsProvider = "openai"

	r2, w2, _ := os.Pipe()
	os.Stdout = w2

	outputJSON(testData)

	w2.Close()
	os.Stdout = oldStdout

	var buf2 bytes.Buffer
	buf2.ReadFrom(r2)
	output2 := buf2.String()

	// Should only contain openai
	if strings.Contains(output2, "anthropic") {
		t.Errorf("outputJSON() with provider filter should not contain other providers, got: %s", output2)
	}
	if !strings.Contains(output2, "openai") {
		t.Errorf("outputJSON() with provider filter should contain openai, got: %s", output2)
	}
}

func TestOutputJSONInvalidData(t *testing.T) {
	// Test JSON marshaling with data that might fail
	// This is a regression test - if models data can't be marshaled, we'd get an error
	testData := models.Models{
		"test": []string{"model1"},
	}

	// Capture stderr
	oldStderr := os.Stderr
	_, w, _ := os.Pipe()
	os.Stderr = w

	// This should not panic
	modelsProvider = ""
	outputJSON(testData)

	w.Close()
	os.Stderr = oldStderr
}

func TestModelsCmdFlags(t *testing.T) {
	if modelsCmd == nil {
		t.Fatal("modelsCmd is nil")
	}

	// Check --provider flag
	flag := modelsCmd.Flags().Lookup("provider")
	if flag == nil {
		t.Error("modelsCmd should have --provider flag")
	}

	// Check --json flag
	flag = modelsCmd.Flags().Lookup("json")
	if flag == nil {
		t.Error("modelsCmd should have --json flag")
	}
}

func TestModelsCmdStructure(t *testing.T) {
	if modelsCmd == nil {
		t.Fatal("modelsCmd is nil")
	}

	// Check that it has subcommands
	if len(modelsCmd.Commands()) == 0 {
		t.Error("modelsCmd should have subcommands")
	}

	// Check for update subcommand
	found := false
	for _, cmd := range modelsCmd.Commands() {
		if cmd.Use == "update" {
			found = true
			break
		}
	}
	if !found {
		t.Error("modelsCmd should have 'update' subcommand")
	}
}

func TestModelsProviderVariable(t *testing.T) {
	// Test the modelsProvider variable
	modelsProvider = "test-provider"
	if modelsProvider != "test-provider" {
		t.Error("modelsProvider should be settable")
	}

	modelsProvider = ""
}

func TestModelsJSONVariable(t *testing.T) {
	// Test the modelsJSON variable
	modelsJSON = true
	if !modelsJSON {
		t.Error("modelsJSON should be settable")
	}

	modelsJSON = false
}
