# Configuration Reference

Create `.agnix.toml` in your project root. All fields are optional with sensible defaults.

## Quick Examples

### Disable Specific Rules

```toml
[rules]
disabled_rules = ["CC-MEM-006", "PE-003", "XP-001"]
```

### Target a Specific Tool

```toml
target = "ClaudeCode"  # Options: Generic, ClaudeCode, Cursor, Codex
```

### Multi-Tool Project

```toml
tools = ["claude-code", "cursor"]
```

## Full Reference

```toml
severity = "Warning"  # Warning, Error, Info
target = "Generic"    # Generic, ClaudeCode, Cursor, Codex

# Multi-tool support (overrides target)
tools = ["claude-code", "cursor"]

exclude = [
  "node_modules/**",
  ".git/**",
  "target/**",
]

[rules]
# Category toggles - all default to true
skills = true              # AS-*, CC-SK-* rules
hooks = true               # CC-HK-* rules
agents = true              # CC-AG-* rules
copilot = true             # COP-* rules
memory = true              # CC-MEM-* rules
plugins = true             # CC-PL-* rules
mcp = true                 # MCP-* rules
prompt_engineering = true  # PE-* rules
xml = true                 # XML-* rules
imports = true             # REF-* rules
cross_platform = true      # XP-* rules
agents_md = true           # AGM-* rules

# Disable specific rules by ID
disabled_rules = ["CC-MEM-006", "PE-003"]

# Version-aware validation (optional)
[tool_versions]
# claude_code = "1.0.0"
# cursor = "0.45.0"

[spec_revisions]
# mcp_protocol = "2025-06-18"
```

## Rule Categories

| Category | Rules | Description |
|----------|-------|-------------|
| skills | AS-*, CC-SK-* | Agent skill validation |
| hooks | CC-HK-* | Hook configuration |
| agents | CC-AG-* | Subagent validation |
| copilot | COP-* | GitHub Copilot instructions |
| memory | CC-MEM-* | Memory/CLAUDE.md |
| plugins | CC-PL-* | Plugin validation |
| mcp | MCP-* | MCP tool validation |
| prompt_engineering | PE-* | Prompt best practices |
| xml | XML-* | XML tag balance |
| imports | REF-* | Import reference validation |
| cross_platform | XP-* | Cross-platform consistency |
| agents_md | AGM-* | AGENTS.md validation |

## Target Filtering

When `target` is set:
- **ClaudeCode** or **Generic**: All rules enabled
- **Cursor** or **Codex**: CC-* rules disabled

## Version-Aware Validation

When versions are not pinned, agnix uses defaults and adds assumption notes. Pin versions for precise validation:

```toml
[tool_versions]
claude_code = "1.0.0"
```

---

## Output Formats

### Text (default)

```bash
agnix .
```

Human-readable colored output with context.

### JSON

```bash
agnix --format json . > results.json
```

```json
{
  "version": "0.3.0",
  "files_checked": 5,
  "diagnostics": [
    {
      "level": "error",
      "rule": "AS-004",
      "file": "SKILL.md",
      "line": 3,
      "column": 1,
      "message": "Invalid name 'Review-Code'",
      "suggestion": "Use lowercase letters and hyphens only"
    }
  ],
  "summary": {
    "errors": 1,
    "warnings": 0,
    "info": 0
  }
}
```

### SARIF

```bash
agnix --format sarif . > results.sarif
```

Full SARIF 2.1.0 compliance for GitHub Code Scanning.

---

## GitHub Action

### Inputs

| Input | Description | Default |
|-------|-------------|---------|
| `path` | Path to validate | `.` |
| `strict` | Treat warnings as errors | `false` |
| `target` | Target tool | `generic` |
| `config` | Path to .agnix.toml | |
| `format` | Output format | `text` |
| `verbose` | Verbose output | `false` |
| `version` | agnix version | `latest` |
| `build-from-source` | Build from source | `false` |
| `fail-on-error` | Fail on errors | `true` |

### Outputs

| Output | Description |
|--------|-------------|
| `result` | success or failure |
| `errors` | Error count |
| `warnings` | Warning count |
| `sarif-file` | SARIF file path |

### Examples

**Basic:**

```yaml
- uses: avifenesh/agnix@v0
```

**Strict with target:**

```yaml
- uses: avifenesh/agnix@v0
  with:
    target: 'claude-code'
    strict: 'true'
```

**SARIF upload:**

```yaml
- uses: avifenesh/agnix@v0
  id: agnix
  with:
    format: 'sarif'

- uses: github/codeql-action/upload-sarif@v3
  with:
    sarif_file: ${{ steps.agnix.outputs.sarif-file }}
```

**Conditional failure:**

```yaml
- uses: avifenesh/agnix@v0
  id: validate
  with:
    fail-on-error: 'false'

- if: steps.validate.outputs.errors > 0
  run: |
    echo "Found ${{ steps.validate.outputs.errors }} errors"
    exit 1
```

---

## Pre-commit Hook

Integrate agnix into your pre-commit workflow.

### Installation

Add to `.pre-commit-config.yaml`:

```yaml
repos:
  - repo: https://github.com/avifenesh/agnix
    rev: v0.3.0
    hooks:
      - id: agnix
```

### Available Hooks

| Hook ID | Description |
|---------|-------------|
| `agnix` | Validate configs (strict mode) |
| `agnix-fix` | Validate and auto-fix |

### With Auto-fix

```yaml
repos:
  - repo: https://github.com/avifenesh/agnix
    rev: v0.3.0
    hooks:
      - id: agnix-fix
```

### Requirements

The `agnix` binary must be installed and available in PATH:

```bash
cargo install agnix-cli
```
