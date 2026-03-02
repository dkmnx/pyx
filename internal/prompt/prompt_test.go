package prompt

import (
	"testing"
)

func TestInputErrorTypes(t *testing.T) {
	tests := []struct {
		name     string
		err      *InputError
		expected string
	}{
		{
			name:     "empty API key error",
			err:      ErrEmptyAPIKey,
			expected: "API key cannot be empty",
		},
		{
			name:     "empty password error",
			err:      ErrEmptyPassword,
			expected: "password cannot be empty",
		},
		{
			name:     "cancelled error",
			err:      ErrCancelled,
			expected: "operation cancelled",
		},
		{
			name:     "invalid provider error",
			err:      ErrInvalidProvider,
			expected: "invalid provider selection",
		},
		{
			name:     "invalid package manager error",
			err:      ErrInvalidPackageManager,
			expected: "invalid package manager selection",
		},
		{
			name:     "custom error",
			err:      &InputError{Message: "custom error message"},
			expected: "custom error message",
		},
		{
			name:     "empty error message",
			err:      &InputError{Message: ""},
			expected: "",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			if tt.err.Error() != tt.expected {
				t.Errorf("Error() = %q, expected %q", tt.err.Error(), tt.expected)
			}
		})
	}
}

func TestErrVariablesNotNil(t *testing.T) {
	if ErrEmptyAPIKey == nil {
		t.Error("ErrEmptyAPIKey should not be nil")
	}
	if ErrEmptyPassword == nil {
		t.Error("ErrEmptyPassword should not be nil")
	}
	if ErrCancelled == nil {
		t.Error("ErrCancelled should not be nil")
	}
	if ErrInvalidProvider == nil {
		t.Error("ErrInvalidProvider should not be nil")
	}
	if ErrInvalidPackageManager == nil {
		t.Error("ErrInvalidPackageManager should not be nil")
	}
}

func TestInputErrorError(t *testing.T) {
	// Test InputError.Error() method directly
	err := &InputError{Message: "test error"}
	msg := err.Error()
	if msg != "test error" {
		t.Errorf("InputError.Error() = %q, expected %q", msg, "test error")
	}

	// Test with empty message
	emptyErr := &InputError{Message: ""}
	if emptyErr.Error() != "" {
		t.Errorf("InputError.Error() with empty message = %q, expected %q", emptyErr.Error(), "")
	}
}

func TestInputErrorType(t *testing.T) {
	// Test that InputError implements error interface
	var err error = ErrEmptyAPIKey
	if err == nil {
		t.Error("ErrEmptyAPIKey should implement error interface")
	}

	// Test error type assertion
	var inputErr *InputError
	inputErr = ErrEmptyAPIKey
	if inputErr == nil {
		t.Error("ErrEmptyAPIKey should be *InputError type")
	}
}

func TestAllErrorMessages(t *testing.T) {
	// Comprehensive test of all error messages
	expectedMessages := map[string]string{
		"ErrEmptyAPIKey":           "API key cannot be empty",
		"ErrEmptyPassword":         "password cannot be empty",
		"ErrCancelled":             "operation cancelled",
		"ErrInvalidProvider":       "invalid provider selection",
		"ErrInvalidPackageManager": "invalid package manager selection",
	}

	errors := map[string]*InputError{
		"ErrEmptyAPIKey":           ErrEmptyAPIKey,
		"ErrEmptyPassword":         ErrEmptyPassword,
		"ErrCancelled":             ErrCancelled,
		"ErrInvalidProvider":       ErrInvalidProvider,
		"ErrInvalidPackageManager": ErrInvalidPackageManager,
	}

	for name, expected := range expectedMessages {
		err := errors[name]
		if err == nil {
			t.Errorf("%s is nil", name)
			continue
		}
		if err.Error() != expected {
			t.Errorf("%s.Error() = %q, expected %q", name, err.Error(), expected)
		}
	}
}

// Note: The interactive prompt functions (PromptProvider, PromptAPIKey, etc.)
// use the tap library for user interaction and cannot be easily unit tested
// without interface abstraction or integration testing.
// These functions would require integration tests or mocking at the tap level.
