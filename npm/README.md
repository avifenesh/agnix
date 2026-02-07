# agnix

Linter for AI agent configurations. Validates SKILL.md, CLAUDE.md, hooks, MCP, and more.

**139 rules** | **Real-time validation** | **Auto-fix** | **Multi-tool support**

## Installation

```bash
npm install -g agnix
```

Or run directly with npx:

```bash
npx agnix .
```

## Usage

### Command Line

```bash
# Lint current directory
agnix .

# Lint specific file
agnix CLAUDE.md

# Auto-fix issues
agnix --fix .

# JSON output
agnix --format json .

# Target specific tool
agnix --target cursor .
```

### Node.js API

```javascript
const agnix = require('agnix');

// Async lint
const result = await agnix.lint('./');
console.log(result);

// Sync run
const { stdout, exitCode } = agnix.runSync(['--version']);

// Get version
console.log(agnix.version());
```

## Supported Files

| File | Tool |
|------|------|
| `SKILL.md` | Claude Code |
| `CLAUDE.md`, `CLAUDE.local.md`, `AGENTS.md`, `AGENTS.local.md`, `AGENTS.override.md` | Claude Code, Codex |
| `.claude/settings.json`, `.claude/settings.local.json` | Claude Code |
| `plugin.json` | Claude Code |
| `*.mcp.json`, `mcp.json`, `mcp-*.json` | All |
| `.github/copilot-instructions.md`, `.github/instructions/*.instructions.md` | GitHub Copilot |
| `.cursor/rules/*.mdc`, `.cursorrules` | Cursor |

## Options

```
agnix [OPTIONS] [PATH] [COMMAND]

Commands:
  validate   Validate agent configs
  init       Initialize config file
  eval       Evaluate rule efficacy against labeled test cases
  telemetry  Manage telemetry settings (opt-in usage analytics)
  schema     Output JSON Schema for configuration files

Common options:
  -s, --strict
  -t, --target <generic|claude-code|cursor|codex>
  -c, --config <CONFIG>
      --fix
      --dry-run
      --fix-safe
      --format <text|json|sarif>
  -w, --watch
  -v, --verbose
  -V, --version
  -h, --help
```

Run `agnix --help` for the full command reference.

## Links

- [GitHub Repository](https://github.com/avifenesh/agnix)
- [Validation Rules](https://github.com/avifenesh/agnix/blob/main/knowledge-base/VALIDATION-RULES.md)
- [VS Code Extension](https://marketplace.visualstudio.com/items?itemName=avifenesh.agnix)

## License

MIT OR Apache-2.0
