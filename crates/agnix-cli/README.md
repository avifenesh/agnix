# agnix

CLI for [agnix](https://github.com/avifenesh/agnix) - the agent configuration linter.

Validates agent configs across Claude Code, Codex/AGENTS.md, Cursor, GitHub Copilot, and MCP.

## Installation

```bash
cargo install agnix-cli
```

## Usage

```bash
# Validate current directory
agnix .

# Validate specific path
agnix /path/to/project

# Target specific tool
agnix --target claude-code .

# Output as SARIF for CI integration
agnix --format sarif .

# Auto-fix issues
agnix --fix .

# Generate .agnix.toml schema
agnix schema --output schemas/agnix.json
```

## Supported Configurations

- Agent skills (`SKILL.md`)
- Claude memory and instructions (`CLAUDE.md`, `CLAUDE.local.md`, `AGENTS.md`, `AGENTS.local.md`, `AGENTS.override.md`)
- Claude settings/hooks (`.claude/settings.json`, `.claude/settings.local.json`)
- Agent files (`agents/*.md`, `.claude/agents/*.md`)
- Plugins (`plugin.json`)
- MCP (`*.mcp.json`, `mcp.json`, `mcp-*.json`)
- GitHub Copilot (`.github/copilot-instructions.md`, `.github/instructions/*.instructions.md`)
- Cursor (`.cursor/rules/*.mdc`, `.cursorrules`)

## Commands

- `agnix [path]` / `agnix validate [path]` - Validate configs
- `agnix init` - Generate starter `.agnix.toml`
- `agnix eval <manifest.yaml>` - Evaluate rule efficacy against labeled fixtures
- `agnix telemetry [status|enable|disable]` - Manage opt-in telemetry
- `agnix schema [--output file]` - Output JSON Schema for `.agnix.toml`

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
