package prompt

import (
	"context"
	"sort"
	"strings"

	"github.com/dkmnx/ply/internal/providers"
	"github.com/yarlson/tap"
)

func PromptProvider(ctx context.Context) (string, error) {
	providerNames := providers.Names()
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

	result := tap.Autocomplete(ctx, tap.AutocompleteOptions{
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

func PromptAPIKey(ctx context.Context) (string, error) {
	result := tap.Password(ctx, tap.PasswordOptions{
		Message: "Enter API key:",
	})

	if result == "" {
		return "", ErrEmptyAPIKey
	}

	return result, nil
}

func PromptPassword(ctx context.Context, message string) (string, error) {
	result := tap.Password(ctx, tap.PasswordOptions{
		Message: message,
	})

	if result == "" {
		return "", ErrEmptyPassword
	}

	return result, nil
}

func PromptNewPassword(ctx context.Context) (string, error) {
	tap.Message("Choose a password to encrypt your master key.")
	tap.Message("This password will be required each time you use ply.")

	for {
		password := tap.Password(ctx, tap.PasswordOptions{
			Message: "Enter password:",
		})

		if password == "" {
			continue
		}

		confirm := tap.Password(ctx, tap.PasswordOptions{
			Message: "Confirm password:",
		})

		if password == confirm {
			return password, nil
		}

		tap.Message("Passwords do not match. Please try again.")
	}
}

func Confirm(ctx context.Context, message string) bool {
	return tap.Confirm(ctx, tap.ConfirmOptions{
		Message: message,
	})
}

var (
	ErrEmptyAPIKey     = &InputError{Message: "API key cannot be empty"}
	ErrEmptyPassword   = &InputError{Message: "password cannot be empty"}
	ErrCancelled       = &InputError{Message: "operation cancelled"}
	ErrInvalidProvider = &InputError{Message: "invalid provider selection"}
)

type InputError struct {
	Message string
}

func (e *InputError) Error() string {
	return e.Message
}
