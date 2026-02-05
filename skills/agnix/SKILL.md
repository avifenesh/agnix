---
name: agnix
triggers:
  - /agnix
  - /validate-config
  - /lint-agents
description: Use when validating agent config files (SKILL.md, CLAUDE.md, AGENTS.md, hooks, MCP, plugins). Catches issues before they break your workflow.
tools:
  - Bash(agnix:*)
  - Bash(cargo:*)
  - Read
  - Glob
  - Grep
---

# agnix - Agent Config Linter

Validate agent configurations before they break your workflow.

## Quick Reference

| Command | Description |
|---------|-------------|
| `agnix .` | Validate current project |
| `agnix --fix .` | Auto-fix issues |
| `agnix --strict .` | Treat warnings as errors |
| `agnix --target claude-code .` | Only Claude Code rules |
| `agnix --watch .` | Watch mode - re-validate on changes |
| `agnix --format json .` | JSON output |
| `agnix --format sarif .` | SARIF for GitHub Code Scanning |

## Supported Files

| File Type | Examples |
|-----------|----------|
| Skills | `SKILL.md` |
| Memory | `CLAUDE.md`, `AGENTS.md` |
| Hooks | `.claude/settings.json` |
| MCP | `*.mcp.json` |
| Cursor | `.cursor/rules/*.mdc` |
| Copilot | `.github/copilot-instructions.md` |

## Execution Steps

When invoked, run these commands:

### 1. Check if agnix is installed

```bash
agnix --version
```

If not installed, install with:
```bash
cargo install agnix-cli
```

### 2. Validate

```bash
agnix .
```

### 3. If issues found, try auto-fix

```bash
agnix --fix .
```

### 4. Re-validate to confirm

```bash
agnix .
```

## Output Format

```
CLAUDE.md:15:1 warning: Generic instruction 'Be helpful' [fixable]
  help: Remove generic instructions. Claude already knows this.

.claude/skills/review/SKILL.md:3:1 error: Invalid name [fixable]
  help: Use lowercase letters and hyphens only

Found 1 error, 1 warning (2 fixable)
```

## Common Fixes

| Issue | Solution |
|-------|----------|
| Invalid skill name | Use lowercase with hyphens: `my-skill` |
| Generic instructions | Remove "be helpful", "be accurate" - Claude knows |
| Missing trigger phrase | Add "Use when..." to skill description |
| Duplicate rules | Remove redundant lines from CLAUDE.md |

## Targets

- `generic` - All rules (default)
- `claude-code` - Claude Code specific
- `cursor` - Cursor specific
- `codex` - Codex CLI specific

## Links

- [Rules Reference](https://github.com/avifenesh/agnix/blob/main/knowledge-base/VALIDATION-RULES.md)
- [Configuration](https://github.com/avifenesh/agnix/blob/main/docs/CONFIGURATION.md)
