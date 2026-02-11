# Documentation

Comprehensive documentation for the ply project.

## User Documentation

### [Getting Started](getting-started.md)

Setup ply for the first time. Covers installation, initial configuration,
and environment setup.

### [Usage Guide](usage.md)

Complete reference for all commands and options. Includes examples for
common workflows.

### [Troubleshooting](troubleshooting.md)

Solutions for common issues including installation, configuration, and
provider problems.

## Developer Documentation

### [Architecture](architecture.md)

System design overview, component interactions, security model, and
data flow diagrams.

### [Contributing](contributing.md)

Guidelines for developers. Covers setup, coding standards, testing,
and pull request process.

## Documentation Standards

All documentation follows these principles:

- **Concise**: Information-dense, no fluff
- **Scannable**: Clear headings, code blocks, tables
- **Contextual**: Relevant examples and use cases
- **Actionable**: Step-by-step instructions where applicable

## Quick Links

| Topic | File |
|-------|------|
| Installation | [Getting Started](getting-started.md) |
| Commands | [Usage Guide](usage.md) |
| Design | [Architecture](architecture.md) |
| Development | [Contributing](contributing.md) |
| Issues | [Troubleshooting](troubleshooting.md) |

## Writing Documentation

When adding new features:

1. Update [Usage Guide](usage.md) with new commands
2. Update [Architecture](architecture.md) if system design changes
3. Add examples to relevant sections
4. Run markdown linting:

```bash
npx markdownlint-cli2 README.md docs/**/*.md --fix
```
