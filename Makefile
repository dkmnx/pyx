.PHONY: build build-prod test lint fmt clean vet install help deps

# Variables
APP_NAME=ply
CMD_DIR=./cmd/ply
BUILD_DIR=./bin
MAIN_CMD=$(CMD_DIR)/main.go

# Build the application
build:
	@echo "Building $(APP_NAME)..."
	@mkdir -p $(BUILD_DIR)
	@go build -o $(BUILD_DIR)/$(APP_NAME) $(MAIN_CMD)
	@echo "Built: $(BUILD_DIR)/$(APP_NAME)"

# Build for production (stripped binary)
build-prod:
	@echo "Building $(APP_NAME) (production)..."
	@mkdir -p $(BUILD_DIR)
	@go build -ldflags="-s -w" -o $(BUILD_DIR)/$(APP_NAME) $(MAIN_CMD)
	@echo "Built: $(BUILD_DIR)/$(APP_NAME)"

# Run tests
test:
	@echo "Running tests..."
	@go test ./... -race -cover

# Run tests with verbose output
test-v:
	@echo "Running tests (verbose)..."
	@go test ./... -race -cover -v

# Run specific test
test-run:
	@echo "Running specific test..."
	@go test ./... -race -cover -v -run $(RUN)

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
clean:
	@echo "Cleaning..."
	@rm -rf $(BUILD_DIR)
	@go clean

# Install locally
install:
	@echo "Installing $(APP_NAME)..."
	@go install $(CMD_DIR)
	@echo "Installed to $$(go env GOPATH)/bin/$(APP_NAME)"

# Run the application
run:
	@echo "Running $(APP_NAME)..."
	@go run $(MAIN_CMD) $(ARGS)

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

# Generate documentation
docs:
	@echo "Generating documentation..."
	@godoc -http=:6060 &

# Show help
help:
	@echo "Available targets:"
	@echo "  build          - Build the application"
	@echo "  build-prod     - Build for production (stripped binary)"
	@echo "  test           - Run all tests"
	@echo "  test-v         - Run all tests with verbose output"
	@echo "  test-run       - Run specific test (set RUN variable)"
	@echo "  lint           - Run linters"
	@echo "  fmt            - Format code"
	@echo "  vet            - Run go vet"
	@echo "  clean          - Clean build artifacts"
	@echo "  install        - Install locally"
	@echo "  run            - Run the application (set ARGS variable)"
	@echo "  check          - Run all checks (fmt, vet, lint, test)"
	@echo "  mod-tidy       - Run go mod tidy"
	@echo "  deps           - Install dependencies (modules and tools)"
	@echo "  deps-outdated  - Check for outdated dependencies"
	@echo "  security       - Run security checks (gosec, govulncheck)"
	@echo "  docs           - Generate documentation (godoc)"
	@echo "  help           - Show this help message"
	@echo ""
	@echo "Examples:"
	@echo "  make test"
	@echo "  make test-run RUN=TestProviderEnvVar"
	@echo "  make run ARGS='--help'"
