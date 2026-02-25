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

### Changed
- Improved error message for unknown providers to guide users to run `ply models update`
- Provider validation now checks format (alphanumeric, hyphens, underscores) and rejects path traversal attempts
- Refactored runSetup() in setup.go for better separation of concerns
- Refactored buildProviderEnv() in root.go for improved testability
- Refactored normalizeEntries() in database.go to extract duplicate detection
- Refactored loadFromFile() in keys.go to extract key data loading
- Refactored getMigrationKey() in keys.go to improve testability

### Security
- Provider name validation prevents path traversal attacks by rejecting names containing `..` or path separators
