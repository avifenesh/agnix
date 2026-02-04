# Change Log

All notable changes to the "agnix" extension will be documented in this file.

## [0.1.0] - 2025-02-04

### Added

- Initial release
- LSP client connecting to agnix-lsp for real-time validation
- Support for all agnix-validated file types:
  - SKILL.md (Agent Skills)
  - CLAUDE.md, AGENTS.md (Claude Code memory)
  - .claude/settings.json (Hooks)
  - plugin.json (Plugins)
  - *.mcp.json (MCP tools)
  - .github/copilot-instructions.md (GitHub Copilot)
  - .cursor/rules/*.mdc (Cursor)
- Status bar indicator showing validation status
- Syntax highlighting for SKILL.md YAML frontmatter
- Commands:
  - `agnix: Restart Language Server`
  - `agnix: Show Output Channel`
- Configuration options:
  - `agnix.lspPath` - Custom path to agnix-lsp binary
  - `agnix.enable` - Enable/disable validation
  - `agnix.trace.server` - Server communication tracing
