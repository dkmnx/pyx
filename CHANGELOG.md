# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Provider name validation against pi's actual model list when using commands
- Validation for `ply <provider>` to prevent invalid or typoed provider names
- Validation for `ply config edit <provider>` to ensure provider names are valid
- Validation for `ply config delete <provider>` to ensure provider names are valid
- Integration tests for end-to-end provider validation

### Changed
- Improved error message for unknown providers to guide users to run `ply models update`
- Provider validation now checks format (alphanumeric, hyphens, underscores) and rejects path traversal attempts

### Security
- Provider name validation prevents path traversal attacks by rejecting names containing `..` or path separators

## [Unreleased] - Examples

### Added
- New visual appearance by default

### Changed
- Drop support for Node 10 and older

### Deprecated
- `--no-more-pizza` flag (use `--pizza-mode=none` instead)

### Removed
- `printReport()` function (use `generateAndPrintReport()` instead)

### Fixed
- Fix crash when parsing malformed JSON

### Security
- Prevent unauthorized access to admin panel

[Unreleased]: https://github.com/username/repo/compare/v1.0.0...HEAD
[v1.0.0]: https://github.com/username/repo/releases/tag/v1.0.0
