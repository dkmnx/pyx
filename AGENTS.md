# Go Development Rules

## First Message

If the user did not give you a concrete task in their first message,
read README.md, then ask which module(s) to work on. Based on the answer, read the relevant README.md files in parallel.

- cmd/*/README.md
- internal/*/README.md
- pkg/*/README.md

## Code Quality

- No `interface{}` or `any` types unless absolutely necessary (prefer specific types)
- Always return concrete types when possible; use interfaces only for behavior abstraction, not for "convenience"
- Prefer exported errors with `var ErrXYZ = errors.New("...")` over string errors
- **NEVER ignore errors** - always handle or return them
- Always ask before removing functionality or code that appears to be intentional
- Never hardcode configuration values; use environment variables, config files, or flags
- Follow Go naming conventions: exported names use PascalCase, unexported use camelCase
- Acronyms in names should be capitalized consistently: `HTTPServer` not `HttpServer`, `XMLParser` not `XmlParser`
- Keep package names short, lowercase, single-word when possible
- Avoid package names like `util`, `common`, `shared` - be specific about what the package does

## Commands

- After code changes (not documentation changes): run checks:

  ```bash
  go vet ./...
  go fmt ./...
  golangci-lint run
  ```

- Fix all errors and warnings before committing
- Run tests: `go test ./... -race -cover`
- Run specific tests: `go test ./path/to/package -run TestSpecificFunction -v`
- NEVER run: `go run main.go` in production contexts
- Build: `go build ./cmd/appname`
- Build for production: `go build -ldflags="-s -w" ./cmd/appname`
- NEVER commit unless user asks

## Module Structure

```text
project/
├── cmd/           # Main applications
│   └── appname/
│       └── main.go
├── internal/      # Private application/library code
│   ├── config/
│   ├── handler/
│   └── service/
├── pkg/           # Public library code
│   └── libname/
├── api/           # API definitions (OpenAPI, protobuf, etc.)
├── scripts/       # Build and utility scripts
├── go.mod
└── go.sum
```

## Error Handling

- Always handle errors explicitly
- Wrap errors with context: `fmt.Errorf("operation failed: %w", err)` or `errors.Wrap(err, "context")`
- Define sentinel errors in package: `var ErrNotFound = errors.New("not found")`
- Use error wrapping judiciously - don't wrap at every level
- For public APIs, document which errors can be returned
- Prefer error checking over panic/recover
- Only use panic for truly unrecoverable conditions (startup failures)

## Concurrency

- Never start goroutines without thinking about how they'll exit
- Always provide a way to cancel goroutines (context.Context)
- Use `sync.WaitGroup` or `errgroup.Group` to manage multiple goroutines
- Prefer channels over shared memory; when sharing, use `sync.Mutex` or `sync.RWMutex`
- Be careful with closures in goroutines - loop variables should be copied as parameters
- Avoid goroutine leaks - always ensure cleanup paths
- Use `context` for cancellation and timeouts

## Testing

- Table-driven tests for multiple test cases:

  ```go
  tests := []struct {
      name    string
      input   string
      want    string
      wantErr bool
  }{
      {"valid input", "test", "result", false},
      {"invalid input", "", "", true},
  }
  for _, tt := range tests {
      t.Run(tt.name, func(t *testing.T) {
          // test implementation
      })
  }
  ```

- Use `t.Cleanup()` for resource cleanup
- Use `testing.TB` interface for test helpers that work with both *testing.T and*testing.B
- Mock interfaces using testify/mock or generate mocks with mockgen
- Keep tests fast; use `t.Parallel()` for independent tests
- Use build tags for integration tests: `//go:build integration`

## Dependencies

- Run `go mod tidy` after adding/removing dependencies
- Pin specific versions for production; use `go get package@version`
- Prefer standard library over external dependencies when possible
- Check for outdated dependencies: `go list -u -m all`
- Verify dependency licenses: `go-licenses check ./...`
- Keep `go.sum` in version control
- Use `replace` directives only for local development; never commit them for production

## Go Modules

- One module per repository (monorepo: subdirectories with their own go.mod)
- Use semantic versioning: `major.minor.patch`
- Pre-release versions: `v1.2.3-rc.1`
- Module path should match import path (e.g., `github.com/user/project`)
- Use `go work` for local multi-module development

## Documentation

- Document exported functions, types, and package behavior
- Use godoc format:

  ```go
  // FunctionName does X and returns Y.
  //
  // The function handles error cases by...
  // Example usage:
  //   result, err := FunctionName(ctx, input)
  func FunctionName(ctx context.Context, input string) (string, error) {
  ```

- Package comments should be in a file named `doc.go`:

  ```go
  // Package example provides utilities for...
  package example
  ```

- Document configuration in README.md
- Keep examples in `*_example_test.go` files

## Linting and Formatting

- Always run `go fmt ./...` before committing
- Use `golangci-lint` with standard rules:

  ```bash
  golangci-lint run --timeout 5m
  ```

- Configure `.golangci.yml` in project root
- Common linters to enable: `go vet`, `staticcheck`, `errcheck`, `gosimple`, `unused`, `gocyclo`
- Keep cyclomatic complexity low (< 15 per function)
- Function length should be reasonable (< 50-100 lines typically)

## Performance

- Profile with `pprof`:

  ```bash
  go test -cpuprofile=cpu.prof -memprofile=mem.prof -bench=.
  go tool pprof cpu.prof
  ```

- Use `benchstat` to compare benchmark results
- Avoid premature optimization; measure first
- Prefer `[]byte` over `string` for binary data
- Use `strings.Builder` for string concatenation in loops
- Be mindful of allocations in hot paths
- Use `sync.Pool` for reusable objects

## Logging

- Use structured logging (e.g., `zerolog`, `zap`, or `slog`)
- Log at appropriate levels: Debug, Info, Warn, Error
- Don't log sensitive information (passwords, tokens, PII)
- Use context for trace IDs and correlation
- Avoid logging in libraries; let applications decide logging strategy

## Configuration

- Use environment variables for deployment config
- Use config files (YAML, TOML, JSON) for complex configuration
- Consider using `cobra` and `viper` for CLI applications
- Support configuration validation on startup
- Document all configuration options
- Never commit secrets or config with real values

## GitHub Issues

When reading issues:

- Always read all comments on the issue
- Use this command to get everything in one call:

  ```bash
  gh issue view <number> --json title,body,comments,labels,state
  ```

When creating issues:

- Add `pkg:*` labels to indicate which package(s) the issue affects
  - Available labels: `pkg:cmd`, `pkg:internal`, `pkg:api`, etc.
- If an issue spans multiple packages, add all relevant labels

When closing issues via commit:

- Include `fixes #<number>` or `closes #<number>` in the commit message
- This automatically closes the issue when the commit is merged

## PR Workflow

- Analyze PRs without pulling locally first
- If the user approves: create a feature branch, pull PR, rebase on main, apply adjustments, commit, merge into main, push, close PR, and leave a comment in the user's tone
- You never open PRs yourself. We work in feature branches until everything is according to the user's requirements, then merge into main, and push.

## Naming Conventions

- **Packages**: lowercase, single word, no underscores or camelCase
  - Good: `http`, `user`, `auth`
  - Bad: `httpServer`, `user_auth`, `httpserver`
- **Interfaces**: usually `-er` suffix for behavior: `Reader`, `Writer`, `Stringer`
  - Exception: one-method interfaces can be named descriptively
- **Constants**: CamelCase (exported) or camelCase (unexported)
- **Variables**: camelCase, short but meaningful
  - Good: `userCount`, `ctx`, `db`
  - Bad: `u`, `x`, `data`
- **Acronyms**: capitalize all letters: `HTTPServer`, `XMLParser`, `userID`, `url`

## Code Organization

- Keep files focused; one primary type per file is a good guideline
- Group related functions together
- Order in file: constants, variables, types, then functions (alphabetical or logical)
- Use `//go:generate` directives for code generation
- Separate interfaces from implementations (e.g., `interface.go` and `impl.go`)

## Adding a New Package/Module

When adding a new package to the project:

### 1. Create Directory Structure

```text
pkg/newpackage/
├── newpackage.go      # Main package file
├── doc.go             # Package documentation
├── newpackage_test.go # Tests
└── README.md          # Package usage documentation
```

### 2. Package Documentation (`doc.go`)

```go
// Package newpackage provides...
//
// Example usage:
//
//   result, err := newpackage.DoSomething(ctx, input)
//   if err != nil {
//       log.Fatal(err)
//   }
package newpackage
```

### 3. Main Implementation

- Export only what needs to be public
- Use unexported types for internal details
- Implement core functionality with comprehensive tests
- Add examples in `*_example_test.go`

### 4. Update Documentation

- `README.md`: Add package to list
- `pkg/newpackage/README.md`: Document usage, API, examples
- `CHANGELOG.md`: Add entry under `## [Unreleased]`

### 5. Tests

- Unit tests for all public functions
- Table-driven tests for multiple scenarios
- Benchmark tests for performance-critical code
- Integration tests (with build tags) if needed

## Adding a New Command (cmd/)

When adding a new CLI command:

### 1. Create Directory

```text
cmd/newcmd/
├── main.go
└── README.md
```

### 2. main.go Structure

```go
package main

import (
    "os"
    "github.com/spf13/cobra"
)

func main() {
    rootCmd := &cobra.Command{
        Use:   "newcmd",
        Short: "Brief description",
        Long:  `Long description...`,
        Run:   run,
    }

    if err := rootCmd.Execute(); err != nil {
        os.Exit(1)
    }
}

func run(cmd *cobra.Command, args []string) {
    // implementation
}
```

## Security

- Never commit secrets, API keys, or credentials
- Use `.env` files (in `.gitignore`) for local development
- Validate all user inputs
- Use `gosec` for security scanning: `gosec ./...`
- Keep dependencies updated; use `go get -u ./...` regularly
- Review dependency security advisories: `govulncheck ./...`
- Use `sqlx` or parameterized queries to prevent SQL injection
- Sanitize output to prevent XSS when generating HTML/JS

## Build and CI

- Use `Makefile` for common commands:

  ```makefile
  build:
      go build -ldflags="-s -w" ./cmd/app

  test:
      go test ./... -race -cover

  lint:
      golangci-lint run

  fmt:
      go fmt ./...

  .PHONY: build test lint fmt
  ```

- Configure CI (GitHub Actions, GitLab CI, etc.) with:
  - Lint checks
  - Tests with coverage
  - Security scans (govulncheck, gosec)
  - Build verification
- Use `docker` for containerized deployments

## Docker

- Use multi-stage builds to minimize image size:

  ```dockerfile
  FROM golang:1.23-alpine AS builder
  WORKDIR /app
  COPY go.* ./
  RUN go mod download
  COPY . .
  RUN go build -ldflags="-s -w" ./cmd/app

  FROM alpine:latest
  COPY --from=builder /app/app /usr/local/bin/app
  CMD ["app"]
  ```

- Use official Go images for building
- Prefer `alpine` or `distroless` for runtime images

## Common Patterns

### HTTP Handler

```go
func (h *Handler) HandleRequest(w http.ResponseWriter, r *http.Request) {
    ctx := r.Context()

    var input Request
    if err := json.NewDecoder(r.Body).Decode(&input); err != nil {
        http.Error(w, "invalid request", http.StatusBadRequest)
        return
    }

    result, err := h.service.DoSomething(ctx, input)
    if err != nil {
        if errors.Is(err, ErrNotFound) {
            http.Error(w, "not found", http.StatusNotFound)
            return
        }
        http.Error(w, "internal error", http.StatusInternalServerError)
        return
    }

    json.NewEncoder(w).Encode(result)
}
```

### Database Transaction

```go
func (r *Repository) Create(ctx context.Context, item Item) error {
    tx, err := r.db.BeginTx(ctx, nil)
    if err != nil {
        return err
    }
    defer tx.Rollback()

    if err := r.createItem(ctx, tx, item); err != nil {
        return err
    }

    return tx.Commit()
}
```

### Worker Pool

```go
func worker(ctx context.Context, jobs <-chan Job, results chan<- Result) {
    for {
        select {
        case <-ctx.Done():
            return
        case job, ok := <-jobs:
            if !ok {
                return
            }
            results <- process(job)
        }
    }
}

func processJobs(ctx context.Context, jobs []Job) []Result {
    jobsChan := make(chan Job, len(jobs))
    results := make(chan Result, len(jobs))

    for i := 0; i < runtime.NumCPU(); i++ {
        go worker(ctx, jobsChan, results)
    }

    for _, job := range jobs {
        jobsChan <- job
    }
    close(jobsChan)

    var resultsSlice []Result
    for i := 0; i < len(jobs); i++ {
        resultsSlice = append(resultsSlice, <-results)
    }

    return resultsSlice
}
```

## Useful Go Tools

- `go vet` - static analysis
- `golangci-lint` - comprehensive linter aggregator
- `gofmt` - code formatter
- `goimports` - import organizer
- `mockgen` - interface mock generator
- `stringer` - string method generator for constants
- `pprof` - profiling
- `benchstat` - benchmark comparison
- `gosec` - security scanner
- `govulncheck` - vulnerability checker
