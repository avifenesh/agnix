# agnix

CLI for [agnix](https://github.com/avifenesh/agnix) - the agent configuration linter.

Validates Skills, Hooks, MCP servers, Memory files, and Plugins for AI coding assistants.

## Installation

```bash
cargo install agnix
```

## Usage

```bash
# Validate current directory
agnix .

# Validate specific path
agnix /path/to/project

# Output as SARIF for CI integration
agnix . --format sarif

# Filter by certainty level
agnix . --certainty high
```

## Supported Configurations

- Claude Code (CLAUDE.md, settings.json, hooks)
- GitHub Copilot (copilot-instructions.md)
- Cursor (.cursorrules)
- MCP servers (server configurations)
- Agent Skills (SKILL.md files)

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
