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

### Validation Pipeline

The validation process follows these steps:

1. **Directory Walking** (sequential) - Uses `ignore` crate to traverse directories
2. **File Collection** - Gathers all relevant file paths with exclusion filtering
3. **Parallel Validation** - Processes files in parallel using rayon
4. **Result Sorting** - Deterministic ordering by severity (errors first) then file path

This architecture ensures fast validation on large projects while maintaining consistent, reproducible output.

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
| CC-AG-001 | ERROR | Missing agent name field |
| CC-AG-002 | ERROR | Missing agent description field |
| CC-AG-003 | ERROR | Invalid model value |
| CC-AG-004 | ERROR | Invalid permission mode |
| CC-AG-005 | ERROR | Referenced skill not found |
| CC-AG-006 | ERROR | Tool/disallowed conflict |
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
target = "Generic"  # Options: Generic, ClaudeCode, Cursor, Codex

[rules]
# Category toggles - enable/disable entire rule categories
skills = true       # AS-*, CC-SK-* rules
hooks = true        # CC-HK-* rules
agents = true       # CC-AG-* rules
memory = true       # CC-MEM-* rules
plugins = true      # CC-PL-* rules
xml = true          # XML-* rules
imports = true      # REF-*, imports::* rules

# Legacy flags (still supported)
generic_instructions = true
frontmatter_validation = true
xml_balance = true
import_references = true

# Disable specific rules by ID
disabled_rules = []  # e.g., ["CC-AG-001", "AS-005"]

[[exclude]]
"node_modules/**"
".git/**"
"target/**"
```

### Target Tool Filtering

When `target` is set to a specific tool, only relevant rules run:
- **ClaudeCode** or **Generic**: All rules enabled
- **Cursor** or **Codex**: CC-* rules disabled (Claude Code specific)

### Rule Categories

| Category | Config Key | Rules | Description |
|----------|------------|-------|-------------|
| Skills | `skills` | AS-*, CC-SK-* | Agent skill validation |
| Hooks | `hooks` | CC-HK-* | Hook configuration validation |
| Agents | `agents` | CC-AG-* | Subagent validation |
| Memory | `memory` | CC-MEM-* | Memory/CLAUDE.md validation |
| Plugins | `plugins` | CC-PL-* | Plugin validation |
| XML | `xml` | xml::* | XML tag balance |
| Imports | `imports` | imports::* | Import reference validation |

## Performance Characteristics

- File I/O is parallelized across all CPU cores
- Directory walking remains sequential to maintain compatibility with `ignore` crate
- Memory usage scales with number of files (diagnostics are collected and sorted)
- Deterministic output guarantees same results across multiple runs
