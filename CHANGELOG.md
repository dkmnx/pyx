# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- **Models cache staleness semantics** (`src/storage/models_cache.rs`): The
  `is_stale` condition now uses `age >= TTL` with full duration precision
  instead of `age.whole_seconds() > TTL`. This has two effects:
  - A cache exactly at TTL is now stale (previously it was fresh).
  - Sub-second age is no longer truncated, so a cache that is a fraction
    of a second past TTL is now stale (previously it stayed fresh for up
    to ~1 extra second). This is the more impactful change and may reduce
    cache hit rates for callers that relied on the old truncation window.
  Both changes are intentional corrections but constitute a semantic
  change.

Initial release of pyx — a CLI for securely managing AI provider API keys for the pi coding agent.

Credentials are encrypted at rest with age, decrypted at runtime, and injected as environment variables into the pi process. No config files or plaintext keys on disk.

- Add, edit, and delete provider API keys (`pyx setup`, `pyx edit`, `pyx delete`)
- Master key stored in OS keyring (Keychain, Credential Manager, gnome-keyring/kwallet) with encrypted file fallback
- `pyx models` to browse available providers and models from pi's catalog
- `pyx list` to show configured providers and their models
- `pyx` runs pi with all provider credentials injected as environment variables
- `-c`/`--continue` and `-r`/`--resume` for pi session management
- `pyx pi install` to install pi with auto-detected package manager
- Shell completion for bash, zsh, fish, and PowerShell
- Cross-platform: Linux, macOS, Windows
