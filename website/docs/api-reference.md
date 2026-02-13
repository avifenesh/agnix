---
title: API Reference
description: "agnix CLI flags, output formats, MCP server tools, and LSP capabilities."
---

# API Reference

## CLI

```bash
agnix [OPTIONS] [PATH]
```

### Options

| Flag | Description |
|------|-------------|
| `[PATH]` | Directory or file to validate (default: `.`) |
| `--target <TOOL>` | Single tool focus (`claude-code`, `cursor`, `codex`, `copilot`) |
| `--tools <TOOLS>` | Comma-separated tool list |
| `--fix` | Apply auto-fixes |
| `--format <FMT>` | Output format: `text` (default), `json`, `sarif` |
| `--strict` | Treat warnings as errors (exit code 1) |
| `--config <PATH>` | Config file path (default: `.agnix.toml`) |
| `--version` | Print version |
| `--help` | Print help |

### Subcommands

| Command | Description |
|---------|-------------|
| `agnix schema [--output FILE]` | Output JSON Schema for `.agnix.toml` |
| `agnix watch [PATH]` | Watch mode - re-validate on file changes |
| `agnix telemetry <status\|enable\|disable>` | Manage telemetry settings |

### Output formats

- **text** - Human-readable terminal output with colors
- **json** - Machine-readable JSON object with diagnostics and summary metadata (e.g. version, files_checked, diagnostics, summary, category, rule_severity, applies_to_tool)
- **sarif** - SARIF format for GitHub Code Scanning integration

## MCP server

```bash
cargo install agnix-mcp
agnix-mcp
```

The MCP server exposes these tools:

| Tool | Description |
|------|-------------|
| `validate_file` | Validate a single configuration file |
| `validate_project` | Validate all config files in a project |
| `get_rules` | List all available validation rules |
| `get_rule_docs` | Get documentation for a specific rule |

## LSP server

```bash
cargo install agnix-lsp
agnix-lsp
```

Supported LSP capabilities:

- `textDocument/publishDiagnostics` - real-time validation
- `textDocument/codeAction` - auto-fix suggestions
- `textDocument/hover` - rule documentation on hover
- `workspace/didChangeConfiguration` - runtime config updates
- `workspace/executeCommand` - project-level validation (`agnix.validateProjectRules` command)

## References

- [SPEC.md](https://github.com/avifenesh/agnix/blob/main/SPEC.md) - full technical specification
- [MCP Protocol](https://modelcontextprotocol.io) - MCP specification
