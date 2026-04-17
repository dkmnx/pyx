# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

Initial release of pyx — a CLI for securely managing AI provider API keys for the pi coding agent.

Credentials are encrypted at rest with age, decrypted at runtime, and injected as environment variables into the pi process. No config files or plaintext keys on disk.

- `pyx init` to initialize encrypted credential store with master key in OS keyring
- `pyx add` and `pyx edit` to manage provider API keys interactively
- `pyx delete` to remove provider credentials
- `pyx models` to browse available providers and models from pi's catalog (with stale flag in JSON output)
- `pyx list` to show configured providers and their models
- `pyx` runs pi with all provider credentials injected as environment variables
- `-c`/`--continue` and `-r`/`--resume` for pi session management
- `pyx pi install` to install pi with auto-detected package manager
- Shell completion for bash, zsh, fish, and PowerShell
- Cross-platform: Linux, macOS, Windows
