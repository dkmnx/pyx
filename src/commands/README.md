# Commands Module

CLI command implementations for pyx.

## Overview

This module contains the command-line interface implementations organized as subcommands.

## Commands

| Command    | File            | Description                        |
| ---------- | --------------- | ---------------------------------- |
| init       | `init.rs`       | Initialize encrypted store         |
| add        | `add.rs`        | Add a provider credential          |
| list       | `list.rs`       | List configured providers          |
| delete     | `delete.rs`     | Remove a provider                  |
| models     | `models.rs`     | List supported AI models           |
| pi         | `pi.rs`         | Pi installation management         |
| reset      | `reset.rs`      | Reset all configuration            |
| completion | `completion.rs` | Generate shell completions         |
| version    | `version.rs`    | Version information                |
| root       | `root.rs`       | Root command and shared options    |

## Module Structure

```text
commands/
├── mod.rs         # Module exports
├── init.rs        # Init command
├── add.rs         # Add command
├── list.rs        # List command
├── delete.rs      # Delete command
├── models.rs      # Models command
├── pi.rs          # Pi command
├── reset.rs       # Reset command
├── completion.rs  # Completion command
├── version.rs     # Version command
└── root.rs        # Root command
```

## Adding a New Command

1. Create `commands/<name>.rs` with command implementation
2. Add module declaration to `commands/mod.rs`
3. Add command to `cli.rs` using clap derive macros
4. Add tests for the new command

## Testing

Commands are tested via integration tests in `tests/` directory.

```bash
just test-integration
```


## Adding a New Command

1. Create `commands/<name>.rs` with command implementation
2. Add module declaration to `commands/mod.rs`
3. Add command to `cli.rs` using clap derive macros
4. Add tests for the new command

## Testing

Commands are tested via integration tests in `tests/` directory.

```bash
just test-integration
```
