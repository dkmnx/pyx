// Package prompt provides interactive prompt utilities using the tap library.
// This file contains the interface abstraction for testable prompt functions.
package prompt

import (
	"context"

	"github.com/yarlson/tap"
)

// TapClient defines the interface for tap library operations.
// This allows mocking for unit tests.
type TapClient interface {
	// Autocomplete shows an autocomplete prompt
	Autocomplete(ctx context.Context, opts tap.AutocompleteOptions) string
	// Password shows a password input prompt
	Password(ctx context.Context, opts tap.PasswordOptions) string
	// Select shows a selection prompt
	Select(ctx context.Context, opts tap.SelectOptions[string]) string
	// Confirm shows a confirmation prompt
	Confirm(ctx context.Context, opts tap.ConfirmOptions) bool
	// Message shows an informational message
	Message(text string, opts tap.MessageOptions)
}

// RealTapClient is the real implementation using the tap library.
type RealTapClient struct{}

// Autocomplete shows an autocomplete prompt using tap.
func (c *RealTapClient) Autocomplete(ctx context.Context, opts tap.AutocompleteOptions) string {
	return tap.Autocomplete(ctx, opts)
}

// Password shows a password input prompt using tap.
func (c *RealTapClient) Password(ctx context.Context, opts tap.PasswordOptions) string {
	return tap.Password(ctx, opts)
}

// Select shows a selection prompt using tap.
func (c *RealTapClient) Select(ctx context.Context, opts tap.SelectOptions[string]) string {
	return tap.Select(ctx, opts)
}

// Confirm shows a confirmation prompt using tap.
func (c *RealTapClient) Confirm(ctx context.Context, opts tap.ConfirmOptions) bool {
	return tap.Confirm(ctx, opts)
}

// Message shows an informational message using tap.
func (c *RealTapClient) Message(text string, opts tap.MessageOptions) {
	tap.Message(text, opts)
}

// defaultClient is the default tap client used by prompt functions.
var defaultClient TapClient = &RealTapClient{}

// SetTapClient sets the tap client for testing purposes.
// This allows injecting mock implementations for unit tests.
func SetTapClient(client TapClient) {
	defaultClient = client
}

// ResetTapClient resets the tap client to the default real implementation.
func ResetTapClient() {
	defaultClient = &RealTapClient{}
}
