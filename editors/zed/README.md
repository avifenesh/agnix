# agnix for Zed

Zed extension for [agnix](https://github.com/avifenesh/agnix) - lint agent configurations before they break your workflow.

Provides real-time validation of AI agent configuration files (CLAUDE.md, AGENTS.md, SKILL.md, `.claude/settings.json`, `*.mcp.json`, `.cursor/rules/*.mdc`, and more) using the `agnix-lsp` language server.

## Features

- Automatic LSP binary download from GitHub releases
- Real-time diagnostics as you type
- Quick-fix code actions for auto-fixable issues
- Hover documentation for configuration fields
- 100 validation rules across 15 categories
- MDC file type support for Cursor rules

## Requirements

- [Zed](https://zed.dev/) editor

The extension automatically downloads the `agnix-lsp` binary -- no manual installation needed.

## Installation

1. Open Zed
2. Open the Extensions panel (Zed > Extensions, or `cmd+shift+x`)
3. Search for "agnix"
4. Click Install

## Configuration

agnix reads configuration from `.agnix.toml` in your project root. See the [Configuration Reference](../../docs/CONFIGURATION.md) for all options.

Example `.agnix.toml`:

```toml
target = "claude-code"
severity = "warning"

[rules]
disabled_rules = ["AS-001"]
```

## Supported File Types

| File Pattern | Type |
|---|---|
| `SKILL.md` | Agent skill definitions |
| `CLAUDE.md`, `CLAUDE.local.md` | Claude Code memory |
| `AGENTS.md`, `AGENTS.local.md`, `AGENTS.override.md` | Agent memory |
| `.claude/settings.json`, `.claude/settings.local.json` | Hook configurations |
| `plugin.json` | Plugin manifests |
| `*.mcp.json`, `mcp.json`, `mcp-*.json` | MCP tool configurations |
| `.github/copilot-instructions.md` | Copilot instructions |
| `.github/instructions/*.instructions.md` | Copilot scoped instructions |
| `.cursor/rules/*.mdc` | Cursor project rules |
| `.cursorrules` | Legacy Cursor rules |
| `.claude/agents/*.md` | Claude agent definitions |

## Troubleshooting

**No diagnostics appearing**

1. Verify the file is a supported type (see table above)
2. Check the Zed log (`cmd+shift+p` > "zed: open log") for LSP errors
3. Ensure the project has a `.git` directory or `.agnix.toml` for root detection

**LSP binary download fails**

The extension downloads the `agnix-lsp` binary from GitHub releases. If the download fails:

1. Check your internet connection
2. Verify you can access https://github.com/avifenesh/agnix/releases
3. Try restarting Zed to trigger a fresh download

**Manual LSP binary**

If automatic download does not work, install `agnix-lsp` manually:

```bash
# npm (easiest)
npm install -g agnix

# Cargo
cargo install agnix-lsp

# Or download from releases
# https://github.com/avifenesh/agnix/releases
```

## License

MIT - see [LICENSE](../../LICENSE-MIT)
