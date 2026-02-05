# Change Log

All notable changes to the "agnix" extension will be documented in this file.

## [0.6.0] - 2026-02-05

### Added

- **Auto-download agnix-lsp** - Binary is automatically downloaded on first use
  - Detects platform (Windows, macOS, Linux) and architecture
  - Downloads from GitHub releases
  - Extracts and installs to extension storage
  - No manual installation required
- **Diagnostics Tree View** - Sidebar panel showing all issues
  - Organized by file with expand/collapse
  - Click to navigate to issue location
  - Error/warning icons with counts
  - Activity bar icon for quick access
- **CodeLens support** - Rule info shown inline above lines with issues
  - Shows error/warning count and rule IDs
  - Click rule ID to view documentation
  - Configurable via `agnix.codeLens.enable` setting
- **Quick-fix preview** - See changes before applying fixes
  - `agnix: Preview Fixes` - Browse and preview all available fixes
  - Shows diff view before applying each fix
  - Confidence indicators (Safe/Review) for each fix
- **Safe fixes only** - `agnix: Fix All Safe Issues` applies only high-confidence fixes
- **Ignore rule command** - `agnix: Ignore Rule in Project` adds rule to `.agnix.toml`
- **Rule documentation** - `agnix: Show Rule Documentation` opens rule docs
- **New commands:**
  - `agnix: Validate Current File` - Validate the active file
  - `agnix: Validate Workspace` - Validate all agent configs in workspace
  - `agnix: Show All Rules` - Browse 100 validation rules by category
  - `agnix: Fix All Issues in File` - Apply all available quick fixes
- **Context menu integration** - Right-click on agent config files
- **Keyboard shortcuts:**
  - `Ctrl+Shift+V` / `Cmd+Shift+V` - Validate current file
  - `Ctrl+Shift+.` / `Cmd+Shift+.` - Fix all issues
  - `Ctrl+Alt+.` / `Cmd+Alt+.` - Fix all safe issues

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
