// Package prompt provides utilities for interactive user input.
package prompt

import (
	"bufio"
	"crypto/rand"
	"encoding/hex"
	"fmt"
	"os"
	"strings"
	"syscall"

	"golang.org/x/term"

	"github.com/spf13/cobra"
)

// Providers is the list of supported providers (from pi coding agent).
var Providers = []string{
	"anthropic",
	"azure-openai-responses",
	"openai",
	"google",
	"groq",
	"cerebras",
	"xai",
	"openrouter",
	"vercel-ai-gateway",
	"zai",
	"mistral",
	"minimax",
	"minimax-cn",
	"huggingface",
	"opencode",
	"kimi-coding",
}

// PromptProvider prompts the user to select a provider from the supported list.
func PromptProvider(cmd *cobra.Command) (string, error) {
	cmd.Printf("Select a provider:\n")
	for i, p := range Providers {
		cmd.Printf("  %d. %s\n", i+1, p)
	}

	for {
		cmd.Print("Enter provider number or name: ")
		input, err := readLine()
		if err != nil {
			return "", fmt.Errorf("failed to read input: %w", err)
		}

		input = strings.TrimSpace(input)

		// Check if input is a number
		var num int
		if _, err := fmt.Sscanf(input, "%d", &num); err == nil {
			if num >= 1 && num <= len(Providers) {
				return Providers[num-1], nil
			}
			cmd.Printf("Invalid number. Please enter 1-%d\n", len(Providers))
			continue
		}

		// Check if input matches a provider name
		for _, p := range Providers {
			if strings.EqualFold(input, p) {
				return p, nil
			}
		}

		cmd.Printf("Invalid provider. Please choose from: %s\n", strings.Join(Providers, ", "))
	}
}

// PromptLabel prompts the user for a label, with a default suggestion.
func PromptLabel(cmd *cobra.Command, provider string) (string, error) {
	suffix := randomSuffix(4)
	defaultLabel := fmt.Sprintf("%s-%s", provider, suffix)

	cmd.Printf("Enter label (default: %s): ", defaultLabel)
	input, err := readLine()
	if err != nil {
		return "", fmt.Errorf("failed to read input: %w", err)
	}

	input = strings.TrimSpace(input)
	if input == "" {
		return defaultLabel, nil
	}

	return input, nil
}

// PromptAPIKey prompts the user for an API key with hidden input.
func PromptAPIKey(cmd *cobra.Command) (string, error) {
	cmd.Print("Enter API key: ")
	input, err := readPassword()
	if err != nil {
		return "", fmt.Errorf("failed to read API key: %w", err)
	}

	input = strings.TrimSpace(input)
	if input == "" {
		return "", fmt.Errorf("API key cannot be empty")
	}

	return input, nil
}

// readLine reads a line of input from stdin.
func readLine() (string, error) {
	reader := bufio.NewReader(os.Stdin)
	line, err := reader.ReadString('\n')
	if err != nil {
		return "", err
	}
	return strings.TrimSuffix(line, "\n"), nil
}

// readPassword reads a password from stdin without echoing.
func readPassword() (string, error) {
	bytePassword, err := term.ReadPassword(int(syscall.Stdin))
	if err != nil {
		return "", err
	}
	fmt.Println() // Print newline after password input
	return string(bytePassword), nil
}

// randomSuffix generates a random hex suffix of the specified byte length.
func randomSuffix(byteLen int) string {
	bytes := make([]byte, byteLen)
	if _, err := rand.Read(bytes); err != nil {
		// Fallback to a simple timestamp-based suffix
		return fmt.Sprintf("%08x", uint64(byteLen))
	}
	return hex.EncodeToString(bytes)
}
