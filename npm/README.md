# agnix

Linter for AI agent configurations. Validates SKILL.md, CLAUDE.md, hooks, MCP, and more.

**100 rules** | **Real-time validation** | **Auto-fix** | **Multi-tool support**

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
| `CLAUDE.md`, `AGENTS.md` | Claude Code, Codex |
| `.claude/settings.json` | Claude Code |
| `plugin.json` | Claude Code |
| `*.mcp.json` | All |
| `.github/copilot-instructions.md` | GitHub Copilot |
| `.cursor/rules/*.mdc` | Cursor |

## Options

```
-t, --target <tool>    Target tool (ClaudeCode, Cursor, Copilot, CodexCli)
-f, --format <format>  Output format (text, json, sarif)
    --fix              Auto-fix issues
-q, --quiet            Only show errors
-v, --verbose          Show detailed output
    --version          Show version
-h, --help             Show help
```

## Links

- [GitHub Repository](https://github.com/avifenesh/agnix)
- [Validation Rules](https://github.com/avifenesh/agnix/blob/main/knowledge-base/VALIDATION-RULES.md)
- [VS Code Extension](https://marketplace.visualstudio.com/items?itemName=avifenesh.agnix)

## License

MIT OR Apache-2.0
