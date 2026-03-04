package prompt

import (
	"context"
	"fmt"
	"sort"
	"strings"
	"unicode"

	"github.com/dkmnx/ply/internal/providers"
	"github.com/yarlson/tap"
)

// Testing mock support
var (
	mockConfirm *bool
)

// Minimum password requirements
const (
	MinPasswordLength  = 8
	MaxPasswordRetries = 3
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
		tap.Cancel("Setup cancelled!")
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
	for {
		result := defaultClient.Password(ctx, tap.PasswordOptions{
			Message: fmt.Sprintf("Enter API key for %s:", provider),
		})

		if result == "" {
			return "", ErrEmptyAPIKey
		}

		if err := validateAPIKey(result, provider); err != nil {
			defaultClient.Message(fmt.Sprintf("Invalid API key: %v. Please try again.", err), tap.MessageOptions{})
			continue
		}

		return result, nil
	}
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

	retries := 0
	for {
		password := defaultClient.Password(ctx, tap.PasswordOptions{
			Message: "Enter password:",
		})

		if password == "" {
			retries++
			if retries >= MaxPasswordRetries {
				tap.Message("Too many failed attempts. Password cannot be empty.")
				return "", ErrCancelled
			}
			defaultClient.Message("Password cannot be empty. Please try again.", tap.MessageOptions{})
			continue
		}

		// Validate password strength
		if err := validatePassword(password); err != nil {
			retries++
			if retries >= MaxPasswordRetries {
				tap.Message(fmt.Sprintf("Too many failed attempts. %v", err))
				return "", ErrCancelled
			}
			defaultClient.Message(fmt.Sprintf("Weak password: %v. Please try again.", err), tap.MessageOptions{})
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
	// Use mock value if set for testing
	if mockConfirm != nil {
		return *mockConfirm
	}

	return defaultClient.Confirm(ctx, tap.ConfirmOptions{
		Message: message,
	})
}

// SetConfirmForTesting sets a mock value for Confirm function (testing only)
func SetConfirmForTesting(value bool) {
	mockConfirm = &value
}

// ResetConfirmForTesting resets the mock value (testing only)
func ResetConfirmForTesting() {
	mockConfirm = nil
}

// Cancel shows a cancellation message.
func Cancel(text string) {
	defaultClient.Cancel(text)
}

// Select shows a selection prompt and returns the selected value.
func Select(ctx context.Context, opts tap.SelectOptions[string]) string {
	return defaultClient.Select(ctx, opts)
}

// Intro shows an intro message.
func Intro(text string) {
	defaultClient.Intro(text)
}

// Outro shows an outro message.
func Outro(text string) {
	defaultClient.Outro(text)
}

// Message shows an informational message.
func Message(text string, opts tap.MessageOptions) {
	defaultClient.Message(text, opts)
}

// validatePassword checks password strength requirements.
// Returns nil if valid, or an error describing the validation failure.
func validatePassword(password string) error {
	if len(password) < MinPasswordLength {
		return ErrPasswordTooShort
	}

	// Check for character diversity
	hasUpper := false
	hasLower := false
	hasDigit := false
	hasSpecial := false

	for _, r := range password {
		switch {
		case unicode.IsUpper(r):
			hasUpper = true
		case unicode.IsLower(r):
			hasLower = true
		case unicode.IsDigit(r):
			hasDigit = true
		case unicode.IsPunct(r) || unicode.IsSymbol(r):
			hasSpecial = true
		}
	}

	// Require at least 3 of 4 character types
	charTypes := 0
	if hasUpper {
		charTypes++
	}
	if hasLower {
		charTypes++
	}
	if hasDigit {
		charTypes++
	}
	if hasSpecial {
		charTypes++
	}

	if charTypes < 3 {
		return ErrPasswordTooSimple
	}

	return nil
}

// validateAPIKey performs basic validation on API key format.
// Returns nil if valid, or an error describing the validation failure.
func validateAPIKey(apiKey string, provider string) error {
	if len(apiKey) < 16 {
		return ErrAPIKeyTooShort
	}

	// Check for common invalid patterns
	if strings.TrimSpace(apiKey) != apiKey {
		return ErrAPIKeyHasWhitespace
	}

	// Check for placeholder patterns
	placeholders := []string{
		"your_api_key",
		"your-api-key",
		"YOUR_API_KEY",
		"YOUR_API_KEY_HERE",
		"insert_key_here",
		"xxx",
		"placeholder",
	}

	lowerKey := strings.ToLower(apiKey)
	for _, placeholder := range placeholders {
		if strings.Contains(lowerKey, placeholder) {
			return ErrAPIKeyLooksLikePlaceholder
		}
	}

	// Note: We don't enforce strict format validation as different providers
	// use different API key formats. The length and placeholder checks above
	// catch the most common mistakes.

	return nil
}

var (
	ErrEmptyAPIKey                = &InputError{Message: "API key cannot be empty"}
	ErrEmptyPassword              = &InputError{Message: "password cannot be empty"}
	ErrCancelled                  = &InputError{Message: "operation cancelled"}
	ErrInvalidProvider            = &InputError{Message: "invalid provider selection"}
	ErrInvalidPackageManager      = &InputError{Message: "invalid package manager selection"}
	ErrPasswordTooShort           = &InputError{Message: fmt.Sprintf("password must be at least %d characters", MinPasswordLength)}
	ErrPasswordTooSimple          = &InputError{Message: "password must contain at least 3 of: uppercase, lowercase, numbers, special characters"}
	ErrAPIKeyTooShort             = &InputError{Message: "API key must be at least 16 characters"}
	ErrAPIKeyHasWhitespace        = &InputError{Message: "API key cannot contain leading or trailing whitespace"}
	ErrAPIKeyLooksLikePlaceholder = &InputError{Message: "API key appears to be a placeholder value"}
)

type InputError struct {
	Message string
}

func (e *InputError) Error() string {
	return e.Message
}
