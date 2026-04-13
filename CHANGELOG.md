# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Added

- `-c`/`--continue` flag to continue the previous pi session (maps to `pi --continue`)
- `-r`/`--resume` flag to interactively select a session to resume (maps to `pi --resume`)
- Provider name validation against pi's model list (prevents typos)
- `pyx models` command to list available providers/models with `update` subcommand
- `pyx list` command showing configured providers with their models
- `pyx delete` command with interactive provider selection
- `pyx reset` command to clear all encrypted data
- `-s`/`--session` flag for pi session support
- DeepSeek and Qwen provider support (via extensions)
- Auto-install pi if missing (with package manager selection)
- Added note about pi extensions for custom providers in documentation
- JSON output support for `pyx version --json` command

### Changed

- Session hint now suggests `pyx -c` instead of `pyx -s <uuid>` — delegates to pi's built-in `--continue` for race-free session resumption
- Removed filesystem scanning for most-recent session (`parse_session_filename`, `find_most_recent_session`)
- **[BREAKING]:** Replaced `ply config` hierarchy with direct commands:
  - `pyx config list` → `pyx list`
  - `pyx config delete <provider>` → `pyx delete`
  - `pyx config edit <provider>` → `pyx edit <provider>`
- **[BREAKING]:** Removed default provider concept - now runs all configured providers by default
- **[BREAKING]:** Removed `pyx init` command - use `pyx setup` instead
- Replaced AES-GCM with age encryption for all credential storage
- Models fetched from GitHub on first use (no longer embedded, enables updates)
- Recovery mode automatically triggered when database exists without master key

### Fixed

- Cross-platform compatibility (Windows paths, executables, PowerShell)
- Provider validation error messages now suggest `pyx models update`
- Documentation corrections for provider environment variable mappings
- Quickstart URL in README now uses correct GitHub releases pattern
- Added missing validation rules documentation for provider names and environment variables

### Security

- Master key stored in OS keyring with password fallback
- Added SecureString type to prevent plaintext credential exposure in memory
- Provider name validation prevents path traversal attacks
- Zeroed master key and API keys after use
