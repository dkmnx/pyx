# ply

A CLI tool for managing AI provider configurations.

## Commands

- `setup` - Initialize ply configuration
- `config` - Manage provider configurations
  - `list` - List all configured providers
  - `edit [provider name or id]` - Edit a provider configuration
  - `delete [provider name or id]` - Delete a provider configuration
- `default [provider name or id]` - Set or view the default provider
- `completion` - Generate shell completion script
- `version` - Print version information

## Installation

```bash
go install github.com/dkmnx/ply/cmd/ply@latest
```

## Development

### Build

```bash
go build ./cmd/ply
```

### Tests

```bash
go test ./... -race -cover
```

### Linting

```bash
golangci-lint run
```

## Project Structure

```text
ply/
├── cmd/
│   └── ply/           # Main application entry point
│       ├── main.go
│       └── README.md
├── internal/
│   ├── cmd/           # CLI commands
│   │   ├── root.go
│   │   ├── setup.go
│   │   ├── config.go
│   │   ├── config_list.go
│   │   ├── config_edit.go
│   │   ├── config_delete.go
│   │   ├── default.go
│   │   ├── completion.go
│   │   └── version.go
│   └── config/        # Configuration management
├── pkg/               # Public libraries
├── go.mod
└── README.md
```
