package prompt

import (
	"testing"
)

func TestInputError(t *testing.T) {
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
			name:     "custom error",
			err:      &InputError{Message: "custom error message"},
			expected: "custom error message",
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

func TestErrEmptyAPIKey(t *testing.T) {
	if ErrEmptyAPIKey == nil {
		t.Error("ErrEmptyAPIKey should not be nil")
	}
	if ErrEmptyAPIKey.Error() != "API key cannot be empty" {
		t.Errorf("ErrEmptyAPIKey.Error() = %q, expected 'API key cannot be empty'", ErrEmptyAPIKey.Error())
	}
}

func TestErrEmptyPassword(t *testing.T) {
	if ErrEmptyPassword == nil {
		t.Error("ErrEmptyPassword should not be nil")
	}
	if ErrEmptyPassword.Error() != "password cannot be empty" {
		t.Errorf("ErrEmptyPassword.Error() = %q, expected 'password cannot be empty'", ErrEmptyPassword.Error())
	}
}
