# agnix - Agent Config Linter

Real-time validation for AI agent configuration files in VS Code.
Install from the [VS Code Marketplace](https://marketplace.visualstudio.com/items?itemName=avifenesh.agnix).

**139 rules** | **Real-time diagnostics** | **Auto-fix** | **Completion** | **Multi-tool support**


## Features

- **Real-time validation** - Diagnostics as you type
- **Context-aware completions** - Frontmatter keys, values, and snippets
- **JSON Schema validation and autocomplete for `.agnix.toml` config files**
- **Validates 139 rules** - From official specs and best practices

- **Diagnostics panel** - Sidebar tree view of all issues by file
- **CodeLens** - Rule info shown inline above problematic lines
- **Quick-fix preview** - See diff before applying fixes
- **Safe fixes** - Apply only high-confidence fixes automatically
- **Ignore rules** - Disable rules directly from the editor
- **Multi-tool** - Claude Code, Cursor, GitHub Copilot, Codex CLI

![Validation in action](assets/vscode-validation.png)

## Supported File Types

| File | Tool | Description |
|------|------|-------------|
| `SKILL.md` | Claude Code | Agent skill definitions |
| `CLAUDE.md`, `AGENTS.md` | Claude Code, Codex | Memory files |
| `.claude/settings.json` | Claude Code | Hook configurations |
| `plugin.json` | Claude Code | Plugin manifests |
| `*.mcp.json` | All | MCP tool configurations |
| `.github/copilot-instructions.md` | GitHub Copilot | Custom instructions |
| `.cursor/rules/*.mdc` | Cursor | Project rules |

## Commands

Access via Command Palette (`Ctrl+Shift+P` / `Cmd+Shift+P`):

| Command | Shortcut | Description |
|---------|----------|-------------|
| `agnix: Validate Current File` | `Ctrl+Shift+V` | Validate active file |
| `agnix: Validate Workspace` | - | Validate all agent configs |
| `agnix: Fix All Issues in File` | `Ctrl+Shift+.` | Apply all available fixes |
| `agnix: Preview Fixes` | - | Browse fixes with diff preview |
| `agnix: Fix All Safe Issues` | `Ctrl+Alt+.` | Apply only safe fixes |
| `agnix: Show All Rules` | - | Browse 139 rules by category |

| `agnix: Show Rule Documentation` | - | Open docs for a rule (via CodeLens) |
| `agnix: Ignore Rule in Project` | - | Add rule to `.agnix.toml` disabled list |
| `agnix: Restart Language Server` | - | Restart the LSP server |
| `agnix: Show Output Channel` | - | View server logs |

## Context Menu

Right-click on agent config files to:
- Validate Current File
- Fix All Issues
- Preview Fixes (with diff)
- Fix All Safe Issues

## Requirements

The `agnix-lsp` binary is **automatically downloaded** on first use. No manual installation required.

If you prefer to install manually:

```bash
# From crates.io
cargo install agnix-lsp

# Or via Homebrew
brew tap avifenesh/agnix && brew install agnix
```

## Settings

All settings can be configured via VS Code's Settings UI or `settings.json`. Changes take effect immediately without restarting the LSP server.

### General Settings

| Setting | Default | Description |
|---------|---------|-------------|
| `agnix.lspPath` | `agnix-lsp` | Path to LSP binary |
| `agnix.enable` | `true` | Enable/disable validation |
| `agnix.codeLens.enable` | `true` | Show CodeLens with rule info |
| `agnix.trace.server` | `off` | Server communication tracing |
| `agnix.severity` | `Warning` | Minimum severity level (Error, Warning, Info) |
| `agnix.target` | `Generic` | Target tool (deprecated, use `tools` instead) |
| `agnix.tools` | `[]` | Tools to validate for (e.g., `["claude-code", "cursor"]`) |

### Rule Categories

Enable or disable validation rule categories. All default to `true`.

| Setting | Rules | Description |
|---------|-------|-------------|
| `agnix.rules.skills` | AS-*, CC-SK-* | Skills validation |
| `agnix.rules.hooks` | CC-HK-* | Hooks configuration |
| `agnix.rules.agents` | CC-AG-* | Agent definitions |
| `agnix.rules.memory` | CC-MEM-* | Memory validation |
| `agnix.rules.plugins` | CC-PL-* | Plugin manifests |
| `agnix.rules.xml` | XML-* | XML balance checking |
| `agnix.rules.mcp` | MCP-* | MCP validation |
| `agnix.rules.imports` | REF-* | Import references |
| `agnix.rules.crossPlatform` | XP-* | Cross-platform compatibility |
| `agnix.rules.agentsMd` | AGM-* | AGENTS.md validation |
| `agnix.rules.copilot` | COP-* | GitHub Copilot instructions |
| `agnix.rules.cursor` | CUR-* | Cursor project rules |
| `agnix.rules.promptEngineering` | PE-* | Prompt engineering |
| `agnix.rules.disabledRules` | - | Specific rule IDs to disable |

### Version Pinning

Pin tool versions for version-aware validation. All default to `null` (use defaults).

| Setting | Example | Description |
|---------|---------|-------------|
| `agnix.versions.claudeCode` | `"1.0.0"` | Claude Code version |
| `agnix.versions.codex` | `"0.1.0"` | Codex CLI version |
| `agnix.versions.cursor` | `"0.45.0"` | Cursor version |
| `agnix.versions.copilot` | `"1.0.0"` | GitHub Copilot version |

### Spec Revisions

Pin specification revisions. All default to `null` (use latest).

| Setting | Example | Description |
|---------|---------|-------------|
| `agnix.specs.mcpProtocol` | `"2025-06-18"` | MCP protocol version |
| `agnix.specs.agentSkills` | `"1.0"` | Agent Skills spec revision |
| `agnix.specs.agentsMd` | `"1.0"` | AGENTS.md spec revision |

### Example settings.json

```json
{
  "agnix.severity": "Error",
  "agnix.tools": ["claude-code", "cursor"],
  "agnix.rules.promptEngineering": false,
  "agnix.rules.disabledRules": ["PE-003"],
  "agnix.versions.claudeCode": "1.0.0"
}
```

### Configuration Priority

VS Code settings take priority over `.agnix.toml`:

1. VS Code settings (highest priority)
2. `.agnix.toml` in workspace root
3. Default values (lowest priority)

## File-Based Configuration

Create `.agnix.toml` in your workspace for team-shared config:

```toml
target = "ClaudeCode"

[rules]
disabled_rules = ["PE-003"]
```

See [configuration docs](https://github.com/avifenesh/agnix/blob/main/docs/CONFIGURATION.md) for all options.

## Troubleshooting

### agnix-lsp not found

The extension automatically downloads agnix-lsp on first use. If automatic download fails:

```bash
# Manual install from crates.io
cargo install agnix-lsp

# Or specify full path in settings
"agnix.lspPath": "/path/to/agnix-lsp"
```

The auto-downloaded binary is stored in the extension's global storage directory.

### No diagnostics appearing

1. Check file type is supported (see table above)
2. Verify status bar shows "agnix" (not "agnix (error)")
3. Run `agnix: Show Output Channel` for error details

## Links

- [agnix on GitHub](https://github.com/avifenesh/agnix)
- [Validation Rules Reference](https://github.com/avifenesh/agnix/blob/main/knowledge-base/VALIDATION-RULES.md)
- [Agent Skills Specification](https://agentskills.io)
- [Model Context Protocol](https://modelcontextprotocol.io)

## License

MIT OR Apache-2.0
