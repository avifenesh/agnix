# agnix Technical Reference

> Linter for agent configs. 74 rules across 5 standards.

## What agnix Validates

| Type | Files | Rules |
|------|-------|-------|
| Skills | SKILL.md | 24 |
| Hooks | settings.json | 11 |
| Memory | CLAUDE.md, AGENTS.md | 10 |
| Agents | agents/*.md | 6 |
| Plugins | plugin.json | 5 |
| MCP | tool definitions | 6 |
| XML | all .md files | 3 |
| References | @imports | 2 |

## Architecture

```
agnix/
├── crates/
│   ├── agnix-core/     # Validation library
│   │   ├── parsers/    # YAML, JSON, Markdown
│   │   ├── schemas/    # Type definitions
│   │   └── rules/      # Validators
│   └── agnix-cli/      # CLI binary
├── knowledge-base/     # 74 rules documented
└── tests/fixtures/     # Test cases
```

## Rule Reference

All rules in `knowledge-base/VALIDATION-RULES.md`

**Rule ID Format:** `[CATEGORY]-[NUMBER]`
- `AS-nnn`: Agent Skills (agentskills.io)
- `CC-SK-nnn`: Claude Code Skills
- `CC-HK-nnn`: Claude Code Hooks
- `CC-MEM-nnn`: Claude Code Memory
- `CC-AG-nnn`: Claude Code Agents
- `CC-PL-nnn`: Claude Code Plugins
- `MCP-nnn`: MCP protocol
- `XML-nnn`: XML validation

## Key Rules

| ID | Severity | Description |
|----|----------|-------------|
| AS-001 | ERROR | YAML frontmatter required |
| AS-004 | ERROR | Name must be kebab-case |
| AS-010 | WARN | Missing trigger phrase |
| CC-SK-006 | ERROR | Dangerous skill without safety flag |
| CC-SK-007 | WARN | Unrestricted Bash access |
| CC-HK-001 | ERROR | Invalid hook event |
| CC-HK-006 | ERROR | Missing command field |
| CC-HK-007 | ERROR | Missing prompt field |
| CC-HK-008 | ERROR | Script file not found |
| CC-HK-009 | WARN | Dangerous command pattern |
| CC-MEM-005 | WARN | Generic instruction detected |
| XML-001 | ERROR | Unclosed XML tag |

## CLI

```bash
agnix .                    # Validate directory
agnix --strict .           # Warnings = errors
agnix --target claude-code # Claude-specific rules
agnix --format json .      # JSON output
```

## Config (.agnix.toml)

```toml
severity = "Warning"
target = "Generic"

[rules]
frontmatter_validation = true
generic_instructions = true

[[exclude]]
"node_modules/**"
```
