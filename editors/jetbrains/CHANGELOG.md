# Changelog

All notable changes to the agnix JetBrains plugin will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-02-05

### Added

- Initial release of agnix JetBrains plugin
- LSP integration using LSP4IJ
- Support for all agnix file types:
  - SKILL.md
  - CLAUDE.md / CLAUDE.local.md
  - AGENTS.md / AGENTS.local.md
  - .claude/settings.json
  - *.mcp.json / mcp.json
  - plugin.json
  - .github/copilot-instructions.md
  - .github/instructions/*.instructions.md
  - .cursor/rules/*.mdc
  - .cursorrules
- Auto-download of LSP binary from GitHub releases
- Platform support: macOS (ARM/Intel), Linux (x86_64/ARM64), Windows (x86_64)
- Settings panel for configuration
- Actions: Restart LSP Server, Validate File, Open Settings
- Notifications for binary status and errors

### Technical Details

- Built with Kotlin 1.9+
- Targets JetBrains IDEs 2023.2+
- Uses LSP4IJ for LSP client support
- Gradle 8.10 with IntelliJ Platform Plugin 2.x

[Unreleased]: https://github.com/avifenesh/agnix/compare/jetbrains-v0.1.0...HEAD
[0.1.0]: https://github.com/avifenesh/agnix/releases/tag/jetbrains-v0.1.0
