# Changelog

All notable changes to the agnix JetBrains plugin are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- Use LSP4IJ `OSProcessStreamConnectionProvider` for language server startup.
- Add LSP4IJ `ServerInstaller` integration for `agnix-lsp` check/install flow.
- Restrict file mappings with `AgnixDocumentMatcher` to avoid false activation on unrelated files.
- Harden download redirects to trusted HTTPS GitHub asset domains.
- Replace custom TAR parser with Apache Commons Compress.

## [0.1.0] - 2026-02-05

### Added

- Initial JetBrains plugin implementation for agnix.
- Support for IntelliJ IDEA, WebStorm, and PyCharm (2023.3+).
- Auto-download and resolution of `agnix-lsp` binary.
- Actions for restart server, validate current file, and settings.
- Settings UI for enable/disable, binary path, auto-download, trace level, and CodeLens.
