# Cross-platform justfile for ply
# Works on Windows, macOS, and Linux

# Set shell for Windows (PowerShell) and Unix systems
set windows-shell := ["pwsh", "-NoProfile", "-Command"]
set shell := ["pwsh", "-NoProfile", "-Command"]

# Variables
app_name := "ply"
cmd_dir := "./cmd/ply"
build_dir := "./bin"
main_cmd := "cmd/ply/main.go"
gopath := `go env GOPATH`

# Version info - cross-platform git commands with fallbacks
version := `git describe --tags --always 2>$null || echo "dev"`
commit := `git rev-parse --short HEAD 2>$null || echo "none"`
date := `Get-Date -AsUTC -Format "yyyy-MM-ddTHH:mm:ssZ"`

ldflags := "-X github.com/dkmnx/ply/internal/cmd.version={{version}} -X github.com/dkmnx/ply/internal/cmd.commit={{commit}} -X github.com/dkmnx/ply/internal/cmd.date={{date}}"

# Build the application
[linux]
[macos]
build:
    @echo "Building {{app_name}}..."
    mkdir -p {{build_dir}}
    go build -ldflags "{{ldflags}}" -o {{build_dir}}/{{app_name}} {{main_cmd}}
    echo "Built: {{build_dir}}/{{app_name}}"

[windows]
build:
    @echo "Building {{app_name}}..."
    @if (!(Test-Path {{build_dir}})) { New-Item -ItemType Directory -Path {{build_dir}} | Out-Null }
    @go build -ldflags "{{ldflags}}" -o {{build_dir}}/{{app_name}}.exe {{main_cmd}}
    @echo "Built: {{build_dir}}/{{app_name}}.exe"

# Build for production (stripped binary)
[linux]
[macos]
build-prod:
    @echo "Building {{app_name}} (production)..."
    mkdir -p {{build_dir}}
    go build -ldflags "-s -w {{ldflags}}" -o {{build_dir}}/{{app_name}} {{main_cmd}}
    echo "Built: {{build_dir}}/{{app_name}}"

[windows]
build-prod:
    @echo "Building {{app_name}} (production)..."
    @if (!(Test-Path {{build_dir}})) { New-Item -ItemType Directory -Path {{build_dir}} | Out-Null }
    @go build -ldflags "-s -w {{ldflags}}" -o {{build_dir}}/{{app_name}}.exe {{main_cmd}}
    @echo "Built: {{build_dir}}/{{app_name}}.exe"

# Run tests
test:
    @echo "Running tests..."
    @go test ./... -race -cover

# Run tests with verbose output
test-v:
    @echo "Running tests (verbose)..."
    @go test ./... -race -cover -v

# Run specific test
test-run RUN:
    @echo "Running specific test..."
    @go test ./... -race -cover -v -run {{RUN}}

# Run linter
lint:
    @echo "Running linters..."
    @golangci-lint run --timeout 5m

# Format code
fmt:
    @echo "Formatting code..."
    @go fmt ./...

# Run go vet
vet:
    @echo "Running go vet..."
    @go vet ./...

# Clean build artifacts
[linux]
[macos]
clean:
    @echo "Cleaning..."
    @rm -rf {{build_dir}}
    @go clean

[windows]
clean:
    @echo "Cleaning..."
    @go clean
    @if exist {{build_dir}} rmdir /s /q {{build_dir}}

# Install locally
[linux]
[macos]
install:
    @echo "Installing {{app_name}}..."
    @go build -ldflags "{{ldflags}}" -o {{gopath}}/bin/{{app_name}} {{main_cmd}}
    @echo "Installed to {{gopath}}/bin/{{app_name}}"

[windows]
install:
    @echo "Installing {{app_name}}..."
    @go build -ldflags "{{ldflags}}" -o {{gopath}}/bin/{{app_name}}.exe {{main_cmd}}
    @echo "Installed to {{gopath}}/bin/{{app_name}}.exe"

# Run the application
run ARGS="":
    @echo "Running {{app_name}}..."
    @go run -ldflags "{{ldflags}}" {{main_cmd}} {{ARGS}}

# All checks before committing
check: fmt vet lint test
    @echo "All checks passed!"

# Run go mod tidy
mod-tidy:
    @echo "Running go mod tidy..."
    @go mod tidy

# Install required dependencies (modules and tools)
deps:
    @echo "Installing dependencies..."
    @go mod download
    @echo "Installing development tools..."
    @go install github.com/golangci/golangci-lint/cmd/golangci-lint@latest
    @go install github.com/securego/gosec/v2/cmd/gosec@latest
    @go install golang.org/x/vuln/cmd/govulncheck@latest
    @echo "Dependencies installed successfully!"

# Check for outdated dependencies
deps-outdated:
    @echo "Checking for outdated dependencies..."
    @go list -u -m all

# Run security checks
security:
    @echo "Running security checks..."
    @echo "Running gosec..."
    @gosec ./...
    @echo "Running govulncheck..."
    @govulncheck ./...

# Show help
help:
    @echo "Available targets:"
    @echo "  build          - Build the application"
    @echo "  build-prod     - Build for production (stripped binary)"
    @echo "  test           - Run all tests"
    @echo "  test-v         - Run all tests with verbose output"
    @echo "  test-run RUN   - Run specific test"
    @echo "  lint           - Run linters"
    @echo "  fmt            - Format code"
    @echo "  vet            - Run go vet"
    @echo "  clean          - Clean build artifacts"
    @echo "  install        - Install locally"
    @echo "  run ARGS       - Run the application"
    @echo "  check          - Run all checks (fmt, vet, lint, test)"
    @echo "  mod-tidy       - Run go mod tidy"
    @echo "  deps           - Install dependencies (modules and tools)"
    @echo "  deps-outdated  - Check for outdated dependencies"
    @echo "  security       - Run security checks (gosec, govulncheck)"
    @echo "  help           - Show this help message"
    @echo ""
    @echo "Examples:"
    @echo "  just test"
    @echo "  just test-run TestProviderEnvVar"
    @echo "  just run --help"
