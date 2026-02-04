# agnix Technical Reference

> Linter for agent configs. 90 rules across 13 categories.

## What agnix Validates

| Type | Files | Rules |
|------|-------|-------|
| Skills | SKILL.md | 24 |
| Hooks | settings.json | 11 |
| Memory (Claude Code) | CLAUDE.md, CLAUDE.local.md | 10 |
| Instructions (Cross-Tool) | AGENTS.md, AGENTS.local.md, AGENTS.override.md | 6 |
| Agents | agents/*.md | 6 |
| Plugins | plugin.json | 5 |
| Prompt Engineering | CLAUDE.md, AGENTS.md | 4 |
| Cross-Platform | AGENTS.md | 3 |
| MCP | tool definitions | 8 |
| XML | all .md files | 3 |
| References | @imports | 2 |
| GitHub Copilot | .github/copilot-instructions.md, .github/instructions/*.instructions.md | 4 |

## Architecture

```
agnix/
├── crates/
│   ├── agnix-core/     # Validation library
│   │   ├── parsers/    # YAML, JSON, Markdown
│   │   ├── schemas/    # Type definitions
│   │   └── rules/      # Validators
│   └── agnix-cli/      # CLI binary
├── knowledge-base/     # 90 rules documented
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
- `AGM-nnn`: AGENTS.md (cross-tool instructions)
- `CC-AG-nnn`: Claude Code Agents
- `COP-nnn`: GitHub Copilot Instructions
- `CC-PL-nnn`: Claude Code Plugins
- `MCP-nnn`: MCP protocol
- `XML-nnn`: XML validation
- `REF-nnn`: @import/reference validation
- `PE-nnn`: Prompt engineering
- `XP-nnn`: Cross-platform compatibility

## Key Rules

| ID | Severity | Description |
|----|----------|-------------|
| AS-001 | ERROR | YAML frontmatter required |
| AS-004 | ERROR | Name must be kebab-case |
| AS-010 | WARN | Missing trigger phrase |
| CC-SK-001 | ERROR | Invalid model value |
| CC-SK-002 | ERROR | Invalid context value |
| CC-SK-003 | ERROR | Context 'fork' requires agent field |
| CC-SK-004 | ERROR | Agent field requires context: fork |
| CC-SK-005 | ERROR | Invalid agent type |
| CC-SK-006 | ERROR | Dangerous skill without safety flag |
| CC-SK-007 | WARN | Unrestricted Bash access |
| CC-SK-008 | ERROR | Unknown tool name |
| CC-SK-009 | WARN | Too many dynamic injections |
| CC-HK-001 | ERROR | Invalid hook event |
| CC-HK-006 | ERROR | Missing command field |
| CC-HK-007 | ERROR | Missing prompt field |
| CC-HK-008 | ERROR | Script file not found |
| CC-HK-009 | WARN | Dangerous command pattern |
| CC-MEM-004 | WARN | Invalid command reference |
| CC-MEM-005 | WARN | Generic instruction detected |
| AGM-003 | WARN | Character limit exceeded (12000 chars) |
| AGM-005 | WARN | Platform features without guard |
| PE-001 | WARN | Critical content in middle |
| PE-002 | WARN | Chain-of-thought on simple task |
| CC-AG-001 | ERROR | Missing agent name field |
| CC-AG-002 | ERROR | Missing agent description field |
| CC-AG-003 | ERROR | Invalid model value |
| CC-AG-004 | ERROR | Invalid permission mode |
| CC-AG-005 | ERROR | Referenced skill not found |
| CC-AG-006 | ERROR | Tool/disallowed conflict |
| CC-PL-001 | ERROR | Plugin manifest not in .claude-plugin/ |
| CC-PL-002 | ERROR | Components inside .claude-plugin/ |
| CC-PL-003 | ERROR | Invalid semver format |
| CC-PL-004 | ERROR | Missing required plugin field |
| CC-PL-005 | ERROR | Empty plugin name |
| XML-001 | ERROR | Unclosed XML tag |
## CLI

```bash
agnix .                    # Validate directory
agnix --strict .           # Warnings = errors
agnix --target claude-code # Claude-specific rules
agnix --fix .              # Apply automatic fixes
agnix --dry-run .          # Preview fixes without modifying files
agnix --fix-safe .         # Only apply safe (HIGH certainty) fixes
agnix --format json .      # JSON output for programmatic consumption
agnix --format sarif .     # SARIF 2.1.0 output for CI/CD
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
mcp = true          # MCP-* rules
prompt_engineering = true  # PE-* rules
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
| GitHub Copilot | `copilot` | COP-* | Copilot instruction validation |
| Memory | `memory` | CC-MEM-* | Memory/CLAUDE.md validation |
| Plugins | `plugins` | CC-PL-* | Plugin validation |
| MCP | `mcp` | MCP-* | MCP tool validation |
| Prompt Engineering | `prompt_engineering` | PE-* | Prompt engineering best practices |
| XML | `xml` | XML-* | XML tag balance |
| Imports | `imports` | REF-* | Import reference validation |

## Performance Characteristics

- File I/O is parallelized across all CPU cores
- Directory walking remains sequential to maintain compatibility with `ignore` crate
- Memory usage scales with number of files (diagnostics are collected and sorted)
- Deterministic output guarantees same results across multiple runs
