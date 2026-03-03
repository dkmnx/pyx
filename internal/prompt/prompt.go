package prompt

import (
	"context"
	"fmt"
	"sort"
	"strings"

	"github.com/dkmnx/ply/internal/providers"
	"github.com/yarlson/tap"
)

func PromptProvider(ctx context.Context) (string, error) {
	providerNames := providers.Names(ctx)
	sort.Strings(providerNames)

	suggest := func(input string) []string {
		if input == "" {
			return providerNames
		}
		var filtered []string
		for _, name := range providerNames {
			if strings.Contains(strings.ToLower(name), strings.ToLower(input)) {
				filtered = append(filtered, name)
			}
		}
		return filtered
	}

	result := defaultClient.Autocomplete(ctx, tap.AutocompleteOptions{
		Message:     "Select a provider:",
		Placeholder: "Start typing...",
		Suggest:     suggest,
		MaxResults:  7,
	})

	if result == "" {
		return "", ErrCancelled
	}

	for _, name := range providerNames {
		if strings.EqualFold(name, result) {
			return name, nil
		}
	}

	matches := suggest(result)
	if len(matches) == 1 {
		return matches[0], nil
	}

	return "", ErrInvalidProvider
}

func PromptAPIKey(ctx context.Context, provider string) (string, error) {
	result := defaultClient.Password(ctx, tap.PasswordOptions{
		Message: fmt.Sprintf("Enter API key for %s:", provider),
	})

	if result == "" {
		return "", ErrEmptyAPIKey
	}

	return result, nil
}

func PromptPassword(ctx context.Context, message string) (string, error) {
	result := defaultClient.Password(ctx, tap.PasswordOptions{
		Message: message,
	})

	if result == "" {
		return "", ErrEmptyPassword
	}

	return result, nil
}

func PromptNewPassword(ctx context.Context) (string, error) {
	defaultClient.Message("Choose a password to encrypt your master key.", tap.MessageOptions{
		Hint: "This password will be required each time you use ply.",
	})

	for {
		password := defaultClient.Password(ctx, tap.PasswordOptions{
			Message: "Enter password:",
		})

		if password == "" {
			continue
		}

		confirm := defaultClient.Password(ctx, tap.PasswordOptions{
			Message: "Confirm password:",
		})

		if password == confirm {
			return password, nil
		}

		defaultClient.Message("Passwords do not match. Please try again.", tap.MessageOptions{})
	}
}

func PromptPackageManager(ctx context.Context) (string, error) {
	result := defaultClient.Select(ctx, tap.SelectOptions[string]{
		Message: "Select a package manager:",
		Options: []tap.SelectOption[string]{
			{Value: "npm", Label: "npm"},
			{Value: "pnpm", Label: "pnpm"},
			{Value: "yarn", Label: "yarn"},
			{Value: "bun", Label: "bun"},
		},
	})

	if result == "" {
		return "", ErrCancelled
	}

	// Validate the selection
	validOptions := []string{"npm", "pnpm", "yarn", "bun"}
	for _, opt := range validOptions {
		if strings.EqualFold(opt, result) {
			return opt, nil
		}
	}

	return "", ErrInvalidPackageManager
}

func Confirm(ctx context.Context, message string) bool {
	return defaultClient.Confirm(ctx, tap.ConfirmOptions{
		Message: message,
	})
}

var (
	ErrEmptyAPIKey           = &InputError{Message: "API key cannot be empty"}
	ErrEmptyPassword         = &InputError{Message: "password cannot be empty"}
	ErrCancelled             = &InputError{Message: "operation cancelled"}
	ErrInvalidProvider       = &InputError{Message: "invalid provider selection"}
	ErrInvalidPackageManager = &InputError{Message: "invalid package manager selection"}
)

type InputError struct {
	Message string
}

func (e *InputError) Error() string {
	return e.Message
}
