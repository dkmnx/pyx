# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Added

- Provider name validation against pi's model list (prevents typos)
- `ply models` command to list available providers/models with `--update` flag
- `ply list` command showing configured providers with their models
- `ply delete` command with interactive provider selection
- `ply reset` command to clear all encrypted data
- `-s`/`--session` flag for pi session support
- DeepSeek and Qwen provider support (via extensions)
- Auto-install pi if missing (with package manager selection)

### Changed

- **[BREAKING]:** Replaced `ply config` hierarchy with direct commands:
  - `ply config list` → `ply list`
  - `ply config delete <provider>` → `ply delete`
  - `ply config edit <provider>` → `ply edit <provider>`
- **[BREAKING]:** Removed default provider concept - now runs all configured providers by default
- **[BREAKING]:** Removed `ply init` command - use `ply setup` instead
- Replaced AES-GCM with age encryption for all credential storage
- Models fetched from GitHub on first use (no longer embedded, enables updates)
- Recovery mode automatically triggered when database exists without master key

### Fixed

- Cross-platform compatibility (Windows paths, executables, PowerShell)
- Provider validation error messages now suggest `ply models update`

### Security

- Master key stored in OS keyring with password fallback
- Added SecureString type to prevent plaintext credential exposure in memory
- Provider name validation prevents path traversal attacks
- Zeroed master key and API keys after use
