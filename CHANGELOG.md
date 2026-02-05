# Changelog

All notable changes to agnix will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Opt-in telemetry module with privacy-first design (#209)
  - Disabled by default, requires explicit `agnix telemetry enable`
  - Tracks aggregate metrics: rule trigger counts, error/warning counts, duration
  - Never collects: file paths, contents, user identity
  - Respects DO_NOT_TRACK, CI, GITHUB_ACTIONS environment variables
  - Feature-gated HTTP client for minimal binary size impact
  - Local event queue for offline storage with automatic retry
- `agnix telemetry` subcommand with status/enable/disable commands
- Comprehensive telemetry documentation in SECURITY.md
- Rule ID validation at collection point (defense-in-depth)

### Changed
- Refactored HooksValidator into standalone validation functions (#212)
  - Extracted 12 validation rules (CC-HK-001 through CC-HK-012) into standalone functions
  - Reduced main validate() method from ~480 to ~210 lines
  - Organized validation into clear phases with documentation
  - Improved maintainability and testability without changing validation behavior

## [0.7.2] - 2026-02-05

### Fixed
- npm package wrapper script now preserved during binary installation
  - Fixes "command not found" error when running `agnix` from npm install
  - Postinstall script backs up and restores wrapper script

## [0.7.1] - 2026-02-05

### Fixed
- VS Code extension LSP installation - now downloads LSP-specific archives (`agnix-lsp-*.tar.gz`)
  - Fixes "chmod: No such file or directory" error on macOS ARM64 and Linux ARM64
  - Added binary existence check before chmod for better error messages
- CC-MEM-006 rule now correctly recognizes positive alternatives before negatives
  - Pattern "DO X, don't do Y" now accepted (previously incorrectly flagged)
  - Example: "Fetch web resources fresh, don't rely on cached data" ✓

### Changed
- Release workflow now builds separate LSP archives for VS Code auto-download

## [0.7.0] - 2026-02-05

### Changed
- Refactored LintConfig internal structure for better maintainability (#214)
  - Introduced RuntimeContext struct to group non-serialized state
  - Introduced RuleFilter trait to encapsulate rule filtering logic
  - Public API remains fully backward compatible

### Added
- FileSystem trait for abstracting file system operations (#213)
  - Enables unit testing validators with MockFileSystem instead of requiring real temp files
  - RealFileSystem delegates to std::fs and file_utils for production use
  - MockFileSystem provides HashMap-based in-memory storage with RwLock for thread safety
  - Support for symlink handling and circular symlink detection
  - Integrated into LintConfig via fs() accessor for dependency injection
- Comprehensive test suite for validation rule coverage (#221)
  - Added exhaustive tests for all valid values in enums and constants
  - Improved test coverage for edge cases and error conditions
  - Fixed test logic to properly reflect tool event requirements

### Performance
- Shared import cache at project validation level reduces redundant parsing (#216)

## [0.3.0] - 2026-02-05

### Added
- Comprehensive config file tests (30+ new tests)
- Performance benchmarks for validation pipeline
- Support for partial config files (only specify fields you need)

### Fixed
- Config now allows partial files - users can specify only `disabled_rules` without all other fields
- Windows path false positives - regex patterns (`\n`, `\s`, `\d`) no longer flagged as Windows paths
- Comma-separated tool parsing - both `Read, Grep` and `Read Write` formats now work
- Git ref depth check - `refs/remotes/origin/HEAD` no longer flagged as deep file paths
- Template placeholder links - `{url}`, `{repoUrl}` placeholders skipped in link validation
- Wiki-style links - single-word links like `[[brackets]]` no longer flagged
