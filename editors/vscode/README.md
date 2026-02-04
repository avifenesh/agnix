# agnix - Agent Config Linter

Real-time validation for agent configuration files in VS Code.

## Features

- Real-time diagnostics for agent configuration files
- Validates 99 rules across multiple configuration types
- Status bar indicator showing validation status
- Syntax highlighting for SKILL.md frontmatter

## Supported File Types

- `SKILL.md` - Agent skill definitions (agentskills.io spec)
- `CLAUDE.md`, `AGENTS.md` - Claude Code memory files
- `.claude/settings.json` - Hook configurations
- `plugin.json` - Plugin manifests
- `*.mcp.json` - MCP tool configurations
- `.github/copilot-instructions.md` - GitHub Copilot instructions
- `.cursor/rules/*.mdc` - Cursor project rules

## Requirements

This extension requires the `agnix-lsp` binary to be installed:

```bash
# From the agnix repository
cargo install --path crates/agnix-lsp

# Or from crates.io (when published)
cargo install agnix-lsp
```

## Extension Settings

This extension contributes the following settings:

- `agnix.lspPath`: Path to the agnix-lsp executable (default: `agnix-lsp`)
- `agnix.enable`: Enable/disable agnix validation (default: `true`)
- `agnix.trace.server`: Traces communication between VS Code and the language server

## Commands

- `agnix: Restart Language Server` - Restart the agnix language server
- `agnix: Show Output Channel` - Show the agnix output channel for debugging

## Configuration

The extension respects `.agnix.toml` configuration files in your workspace:

```toml
severity = "Warning"
target = "ClaudeCode"

[rules]
skills = true
hooks = true
agents = true
mcp = true

exclude = [
  "node_modules/**",
  ".git/**"
]
```

See the [agnix documentation](https://github.com/avifenesh/agnix) for full configuration options.

## Troubleshooting

### agnix-lsp not found

If you see "agnix-lsp not found", ensure the binary is installed and in your PATH:

```bash
# Check if installed
which agnix-lsp  # Unix
where agnix-lsp  # Windows

# Or specify the full path in settings
"agnix.lspPath": "/path/to/agnix-lsp"
```

### No diagnostics appearing

1. Check that the file is a supported type (see above)
2. Verify the language server is running (check status bar)
3. Open the output channel (`agnix: Show Output Channel`) for errors

## Links

- [agnix on GitHub](https://github.com/avifenesh/agnix)
- [Agent Skills Specification](https://agentskills.io)
- [Model Context Protocol](https://modelcontextprotocol.io)

## License

MIT OR Apache-2.0
