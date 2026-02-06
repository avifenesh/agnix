# Changelog

All notable changes to the agnix JetBrains plugin are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.9.1] - 2026-02-06

### Fixed
- Exclude developer-focused markdown files from validation scope.
- Detect positive alternatives after negatives in configuration memory rules (CC-MEM-006).


## [0.7.3] - 2026-02-05

### Fixed

- Replace deprecated URL constructors with URI-based parsing and resolution in binary downloader.
- Replace deprecated settings file chooser APIs to remove scheduled-for-removal usage.
- Use the same plugin icon asset as VS Code in JetBrains plugin metadata.

## [0.1.1] - 2026-02-05

### Fixed

- Re-resolve `agnix-lsp` at startup so first-run installer downloads can launch immediately.
- Keep `.cursorrules` matching global and remove misleading GitHub-specific grouping.
- Add explicit `.cursorrules` path coverage in JetBrains file-type tests.
- Extend plugin compatibility range through JetBrains build `253.*`.

### Changed

- Use LSP4IJ `OSProcessStreamConnectionProvider` for language server startup.
- Add LSP4IJ `ServerInstaller` integration for `agnix-lsp` check/install flow.
- Restrict file mappings with `AgnixDocumentMatcher` to avoid false activation on unrelated files.
- Harden download redirects to trusted HTTPS GitHub asset domains.
- Replace custom TAR parser with Apache Commons Compress.
- Add real JetBrains diagnostics screenshot to plugin and repository docs.

## [0.1.0] - 2026-02-05

### Added

- Initial JetBrains plugin implementation for agnix.
- Support for IntelliJ IDEA, WebStorm, and PyCharm (2023.3+).
- Auto-download and resolution of `agnix-lsp` binary.
- Actions for restart server, validate current file, and settings.
- Settings UI for enable/disable, binary path, auto-download, trace level, and CodeLens.
