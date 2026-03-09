# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Added
- Provider name validation against pi's actual model list when using commands
- Validation for `ply <provider>` to prevent invalid or typoed provider names
- Validation for `ply config edit <provider>` to ensure provider names are valid
- Validation for `ply config delete <provider>` to ensure provider names are valid
- Integration tests for end-to-end provider validation
- Comprehensive unit tests for extracted helper functions
- DeepSeek provider support when `deepseek-provider` extension is present
- Qwen provider support when extension exists
- Input validation for `EncodeCwd` and `DecodeCwd` functions
- Session helper package for finding pi sessions
- `-s`/`--session` flag to ply CLI for session support
- Session hint display after pi exits with resume command
- Package manager selection for pi installation
- Pi dependency check and auto-install
- Reset command to clear all encrypted data
- Confirmation prompt before editing API key in `ply config edit`
- Input validation for password and API key prompts
- Password error recovery mechanism
- Recovery mode when database exists without master key
- Prompts for password when master key exists during init
- Unified settings configuration system
- Cancellation support to model fetching
- Warning message for keyring deletion failures
- Comprehensive input validation for provider names
- Secure master key storage with keyring and password fallback
- `SecureString` type to prevent plaintext credential exposure
- Cache TTL staleness check with 24h expiry
- `ply models` command with remote update support
- Context cancellation support across codebase
- Comprehensive tests for prompt and cmd packages
- SecureBytes implementation
- Atomic save operations

### Changed
- Improved error message for unknown providers to guide users to run `ply models update`
- Provider validation now checks format (alphanumeric, hyphens, underscores) and rejects path traversal attempts
- Refactored runSetup() in setup.go for better separation of concerns
- Refactored buildProviderEnv() in root.go for improved testability
- Refactored normalizeEntries() in database.go to extract duplicate detection
- Refactored loadFromFile() in keys.go to extract key data loading
- Refactored getMigrationKey() in keys.go to improve testability
- Changed `PromptAPIKey` to return `SecureString` for proper memory cleanup
- Use generic error messages to prevent oracle attacks
- Updated Go version to 1.26
- Isolated keyring namespace to prevent test interference
- Fixed autocomplete and select component value capture issues
- Decoupled models fetching from hardcoded pi-mono repository
- Improved provider env var mapping and `SecureString.Equal`
- Removed deprecated `GetAll` function and updated `IsValid`
- Added context cancellation checks to database Save and fixed Windows path handling
- Added nil check to `zeroMasterKey` to prevent panic
- Removed 'ply config' command entirely
- Moved 'ply config delete' to 'ply delete'
- Removed deprecated 'ply config list' command
- Added 'ply list' command showing providers with their models
- Use git commands for cross-platform version info
- Updated c2sp.org/CCTV/age dependency to latest version
- Isolated test data from user data
- Fixed EncodeCwd path separator handling
- Show helpful error when provider key decryption fails
- Use `prompt.Outro` for delete success message
- Added Cancel, Intro, Outro, Select, Message wrappers for prompt package
- Made provider selection interactive with tap for delete command
- Converted Makefile to cross-platform justfile
- Improved code clarity with better naming and comments
- Standardized context propagation across codebase
- Improved setup command user-facing messages
- Removed init command, moved recovery to setup
- Improved user prompt UX
- Init enters recovery mode when database exists without master key
- Replaced AES-GCM with age encryption
- Removed automatic model filtering
- Refactored tests for cmd, pi, and prompt packages
- Resolved cross-platform compatibility and test isolation issues
- Added .exe extension to Windows build outputs
- Made justfile compatible with PowerShell on Windows
- Removed duplicate db.Load() calls
- Simplified validateProvider to return validation result directly
- Replaced prompt library with tap library for interactive prompts
- Improved documentation structure and updated to use just commands
- Added injection of git commit hash as version via ldflags
- Standardized file permissions and path handling
- Properly distinguished keyring vs password-based storage
- Removed leading newline from session hint
- Show session hint command on separate line
- Always show session hint after pi exits
- Fixed formatting issues (gofmt)
- Used SecureString and pass env directly to child commands
- Removed default command tests
- Removed prompt label tests
- Updated tests for provider-keyed model
- Removed label and default model prompts from prompt package
- Removed default provider command
- Operate config commands by provider name only
- Prompt for provider and API key only in setup
- Support multi-provider execution in root command
- Removed default provider file handling
- Simplified database entry model to use provider as unique key
- Parse all providers and models from TypeScript file
- Removed redundant version file
- Fixed double time.Now() call
- Removed warnings and silently fetch models from GitHub
- Removed embedded models, fetch from GitHub on first use
- Externalized embedded models to JSON file
- Passed provider and model as separate flags to pi
- Matched setup output format with config list
- Support XDG Base Directory spec and use fetched providers
- Use cached models in ForProvider and GetAll
- Registered commands directly in their respective files
- Deduplicated line-reading logic into prompt package
- Added delete confirmation and deduplicated constants
- Extracted label/ID lookup to GetEntryByLabelOrID helper

### Deprecated
- Removed 'ply config' command hierarchy (replaced with 'ply delete')
- Removed 'ply config list' command (replaced with 'ply list')

### Removed
- Removed init command entirely
- Removed automatic model filtering feature
- Removed embedded models JSON file
- Removed redundant version file
- Removed deprecated GetAll function
- Removed default provider command
- Removed default provider file handling
- Removed prompt label functionality

### Fixed
- Minimized master key exposure by using byte-based encryption
- Addressed immediate security issues from code review
- Added input validation to EncodeCwd and DecodeCwd
- Used generic error messages to prevent oracle attacks
- Addressed linter issues and added security improvements
- Updated Go version to 1.26 and fixed type errors in tests
- Resolved cross-platform compatibility and test isolation issues
- Added context cancellation checks to database Save
- Fixed Windows path handling in database operations
- Fixed nil pointer panic in zeroMasterKey
- Fixed autocomplete and select component value capture issues
- Fixed EncodeCwd path separator handling on Windows
- Fixed provider key decryption error display
- Fixed double time.Now() call in version handling
- Fixed formatting issues across codebase
- Fixed lint issues in completion.go
- Fixed flag handling when passed to pi
- Fixed nil slice panic in executePi
- Fixed version info retrieval for cross-platform compatibility
- Fixed provider validation error messages
- Fixed SecureString thread-safety and timing attack vulnerabilities
- Fixed master key zeroing after use
- Fixed file permissions for completion files
- Fixed cross-platform path handling with filepath.Join
- Fixed duplicate db.Load() calls
- Fixed markdown linting issues in AGENTS.md
- Fixed stuttering types and function names

### Security
- Provider name validation prevents path traversal attacks by rejecting names containing `..` or path separators
- Minimized master key exposure through byte-based encryption
- Zeroed API key after encryption in storeProviderEntry
- Replaced AES-GCM with age encryption for master key storage
- Stored password in OS keyring instead of plaintext file
- Zeroed master key after initialization and use
- Added SecureString type to prevent plaintext credential exposure
- Added comprehensive input validation for provider names to prevent path traversal
- Added secure master key storage with keyring and password fallback
- Fixed SecureString thread-safety and timing attack vulnerabilities
- Added warning message for keyring deletion failures
- Used secure file permissions for master key and database files
- Added password handling security improvements in keys package
