# Contributing Guide

Guidelines for contributing to ply development.

## Development Setup

### Prerequisites

- Go 1.21 or later
- golangci-lint
- make

### Clone and Build

```bash
git clone https://github.com/dkmnx/ply.git
cd ply
make build
```

### Running Tests

```bash
# Run all tests
make test

# Run with coverage
make test-v

# Run specific test
make test-run RUN=TestProviderEnvVar
```

### Linting and Formatting

```bash
# Format code
make fmt

# Run linters
make lint

# Run all checks
make check
```

## Code Standards

### Go Conventions

- Follow Effective Go guidelines
- Use PascalCase for exported names
- Use camelCase for unexported names
- Acronyms in all caps: `HTTP`, `API`, `ID`

### Error Handling

- Always handle errors explicitly
- Use sentinel errors: `var ErrNotFound = errors.New("not found")`
- Wrap errors with context: `fmt.Errorf("failed to X: %w", err)`

### Documentation

- Document all exported functions
- Use godoc format
- Add examples for complex functions

```go
// FunctionName does X and returns Y.
//
// The function handles error cases by...
func FunctionName() error {
    // implementation
}
```

### Testing

- Use table-driven tests
- Name test cases descriptively
- Use `t.Run()` for sub-tests

```go
tests := []struct {
    name string
    input string
    want string
}{
    {"valid input", "test", "result"},
    {"empty input", "", ""},
}
for _, tt := range tests {
    t.Run(tt.name, func(t *testing.T) {
        // test
    })
}
```

## Project Structure

```text
ply/
├── cmd/ply/          # Application entry point
├── internal/
│   ├── cmd/          # CLI commands
│   ├── crypto/       # Encryption
│   ├── database/     # Storage
│   ├── fs/           # File system
│   └── prompt/       # Interactive input
├── docs/            # Documentation
├── Makefile         # Build targets
└── AGENTS.md        # Development rules
```

## Adding a New Provider

### 1. Update provider list

Edit `internal/prompt/prompt.go`:

```go
var Providers = []string{
    // existing providers...
    "new-provider",
}
```

### 2. Add environment variable mapping

Edit `internal/cmd/root.go`:

```go
envMap := map[string]string{
    // existing mappings...
    "new-provider": "NEW_PROVIDER_API_KEY",
}
```

### 3. Add tests

Update `internal/cmd/root_test.go` with the new provider mapping.

## Adding a New Command

### 1. Create command file

```go
// internal/cmd/newcmd.go
package cmd

import "github.com/spf13/cobra"

var newCmd = &cobra.Command{
    Use:   "newcmd",
    Short: "Brief description",
    Long:  `Long description...`,
    Run:   runNew,
}

func init() {
    rootCmd.AddCommand(newCmd)
}

func runNew(cmd *cobra.Command, args []string) {
    // implementation
}
```

### 2. Add documentation

- Update `docs/usage.md` with command reference
- Add examples to `cmd/ply/README.md`

## Building for Release

```bash
# Build production binary
make build-prod

# Versioned release
VERSION=v1.2.3 make build-prod
```

## Submitting Changes

1. Ensure all checks pass: `make check`
2. Commit changes with descriptive message
3. Push to your fork
4. Create pull request

## Commit Message Format

```text
type(scope): description

- detail 1
- detail 2
```

Types: `feat`, `fix`, `docs`, `chore`, `refactor`, `test`
