// Package prompt provides interactive user input utilities for configuring
// AI provider credentials in the ply CLI tool.
//
// This package handles prompting users for provider selection
// and API keys with secure input handling.
package prompt

import (
	"bufio"
	"fmt"
	"os"
	"sort"
	"strings"
	"syscall"

	"github.com/dkmnx/ply/internal/models"
	"golang.org/x/term"

	"github.com/spf13/cobra"
)

func PromptProvider(cmd *cobra.Command) (string, error) {
	if err := models.FetchAndCache(); err != nil {
		return "", fmt.Errorf("failed to fetch models: %w", err)
	}

	allModels := models.GetAll()
	providerList := make([]string, 0, len(allModels))
	for p := range allModels {
		providerList = append(providerList, p)
	}

	sort.Strings(providerList)

	cmd.Printf("Select a provider:\n")
	for i, p := range providerList {
		cmd.Printf("  %d. %s\n", i+1, p)
	}

	for {
		cmd.Print("Enter provider number or name: ")
		input, err := ReadLine()
		if err != nil {
			return "", fmt.Errorf("failed to read input: %w", err)
		}

		input = strings.TrimSpace(input)

		// Check if input is a number
		var num int
		if _, err := fmt.Sscanf(input, "%d", &num); err == nil {
			if num >= 1 && num <= len(providerList) {
				return providerList[num-1], nil
			}
			cmd.Printf("Invalid number. Please enter 1-%d\n", len(providerList))
			continue
		}

		// Check if input matches a provider name
		for _, p := range providerList {
			if strings.EqualFold(input, p) {
				return p, nil
			}
		}

		cmd.Printf("Invalid provider. Please choose from: %s\n", strings.Join(providerList, ", "))
	}
}

// PromptAPIKey prompts the user for an API key with hidden terminal input.
//
// The API key is read without echoing to the terminal for security.
// Empty input is rejected and prompts are repeated until valid input is received.
//
// Returns the API key string or an error if input fails.
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

// ReadLine reads a line of input from stdin.
func ReadLine() (string, error) {
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
