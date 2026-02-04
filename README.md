# agnix

> The nginx of agent configs

Validate agent specifications across Claude Code, Cursor, Codex, and beyond.

**Validates:** Skills • MCP • Hooks • Memory • Agents • Plugins

```bash
agnix .
```

## Features

- ✅ **Agent Skills** - Validates SKILL.md format (agentskills.io spec + CC-SK-001 to CC-SK-009)
- ✅ **Claude Code** - CLAUDE.md (and variants: CLAUDE.local.md), hooks, subagents, plugins
- ✅ **Subagents** - Agent frontmatter validation (CC-AG-001 to CC-AG-006)
- ✅ **GitHub Copilot** - Copilot instruction file validation (COP-001 to COP-004)
- ✅ **Cursor Project Rules** - .cursor/rules/*.mdc validation (CUR-001 to CUR-006)
- ✅ **Plugins** - Plugin manifest validation (CC-PL-001 to CC-PL-005)
- ✅ **Generic Instructions** - Detects redundant "be helpful" patterns
- ✅ **XML Balance** - Ensures tags are properly closed
- ✅ **@imports** - Validates file references exist
- ✅ **Hooks** - Event and config validation (CC-HK-001 to CC-HK-011)
- ✅ **Parallel Validation** - Fast processing of large projects using rayon
- ✅ **MCP Tools** - Schema and tool validation (MCP-001 to MCP-006)
- ✅ **AGENTS.md** - Cross-tool instruction validation (AGM-001 to AGM-006)
- ✅ **Cross-Platform** - AGENTS.md validation, platform-specific feature detection (XP-001 to XP-003)
- ✅ **Prompt Engineering** - Validates prompt best practices (PE-001 to PE-004)
- 🚧 **LSP Server** - Real-time diagnostics (coming soon)

## Installation

### From source

```bash
cargo install --path crates/agnix-cli
```

### From crates.io (coming soon)

```bash
cargo install agnix
```

## Quick Start

```bash
# Validate current directory
agnix .

# Validate specific path
agnix /path/to/project

# Strict mode (warnings = errors)
agnix --strict .

# Target specific tool
agnix --target claude-code .

# Apply automatic fixes
agnix --fix .

# Preview fixes without modifying files
agnix --dry-run .

# Apply only safe (HIGH certainty) fixes
agnix --fix-safe .

# JSON output format (for programmatic consumption)
agnix --format json .

# SARIF output format (for CI/CD and GitHub Code Scanning)
agnix --format sarif .

# Generate config file
agnix init

# Evaluate rule efficacy
agnix eval tests/eval.yaml
```

## Output

```
Validating: .

CLAUDE.md:15:1 warning: Generic instruction 'Be helpful and accurate' [fixable]
  help: Remove generic instructions. Claude already knows this.

.claude/skills/review/SKILL.md:3:1 error: Invalid name 'Review-Code' [fixable]
  help: Use lowercase letters and hyphens only (e.g., 'code-review')

.claude/skills/review/SKILL.md:4:8 error: Unknown model 'gpt-4'
  help: Use: sonnet, opus, haiku, inherit

.claude/agents/researcher.md:1:0 error: Agent frontmatter is missing required 'name' field
  help: Add 'name: your-agent-name' to frontmatter

.claude-plugin/plugin.json:1:0 error: Missing required field 'version'
  help: Add 'version' field with semver format (e.g., "1.0.0")

────────────────────────────────────────────────────────────
Found 4 errors, 1 warning
  2 issues are automatically fixable

hint: Run with --fix to apply fixes
```

## Output Formats

### JSON Output Format

For programmatic consumption and CI/CD integration, use the `--format json` option:

```bash
agnix --format json . > results.json
```

Features:
- **Simple, human-readable structure** - Easy to parse and integrate with custom tooling
- **Version tracking** - Includes agnix version for compatibility checks
- **Summary statistics** - Quick counts of errors, warnings, and info messages
- **Cross-platform paths** - Automatically normalizes Windows backslashes to forward slashes
- **Relative paths** - File paths are relative to the validation base directory
- **Proper exit codes** - Returns exit code 1 if errors are found (0 for success)
- **Fix flags** - `--fix`, `--dry-run`, and `--fix-safe` are only supported with text output

Example JSON output structure:
```json
{
  "version": "0.x.x",
  "files_checked": 5,
  "diagnostics": [
    {
      "level": "error",
      "rule": "AS-004",
      "file": "SKILL.md",
      "line": 3,
      "column": 1,
      "message": "Invalid name 'Review-Code'",
      "suggestion": "Use lowercase letters and hyphens only (e.g., 'code-review')"
    }
  ],
  "summary": {
    "errors": 1,
    "warnings": 0,
    "info": 0
  }
}
```

### SARIF Output Format

For CI/CD integration and GitHub Code Scanning, use the `--format sarif` option:

```bash
agnix --format sarif . > results.sarif
```

Features:
- **Full SARIF 2.1.0 compliance** - Compatible with GitHub Code Scanning and other SARIF tools
- **84 validation rules** - All rules included in `driver.rules` with help URIs linking to documentation
- **Proper exit codes** - Returns exit code 1 if errors are found (0 for success)
- **Cross-platform paths** - Automatically normalizes Windows backslashes to forward slashes
- **Relative paths** - File paths are relative to the validation base directory
- **Fix flags** - `--fix`, `--dry-run`, and `--fix-safe` are only supported with text output

Example SARIF output structure:
```json
{
  "$schema": "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/main/sarif-2.1/schema/sarif-schema-2.1.0.json",
  "version": "2.1.0",
  "runs": [{
    "tool": {
      "driver": {
        "name": "agnix",
        "version": "0.x.x",
        "informationUri": "https://github.com/avifenesh/agnix",
        "rules": [...]
      }
    },
    "results": [...]
  }]
}
```



### CI/CD Integration Examples

```bash
# JSON format - Parse with jq for custom processing
agnix --format json . | jq '.summary.errors'

# SARIF format - GitHub Actions integration
agnix --format sarif . > results.sarif
```

## GitHub Action

Use the official agnix GitHub Action for seamless CI/CD integration:

```yaml
- name: Validate agent configs
  uses: avifenesh/agnix@v0.1.0
  with:
    target: 'claude-code'
```

### Action Inputs

| Input | Description | Default |
|-------|-------------|---------|
| `path` | Path to validate | `.` |
| `strict` | Treat warnings as errors | `false` |
| `target` | Target tool (generic, claude-code, cursor, codex) | `generic` |
| `config` | Path to .agnix.toml config file | |
| `format` | Output format (text, json, sarif) | `text` |
| `verbose` | Verbose output | `false` |
| `version` | agnix version to use | `latest` |
| `build-from-source` | Build from source instead of downloading (requires Rust) | `false` |
| `fail-on-error` | Fail if validation errors found (set false to check result output) | `true` |

**Note:** The action requires `jq` for JSON parsing (pre-installed on GitHub-hosted runners).

### Action Outputs

| Output | Description |
|--------|-------------|
| `result` | Validation result (success or failure) |
| `errors` | Number of errors found |
| `warnings` | Number of warnings found |
| `sarif-file` | Path to SARIF file (if format=sarif) |

### Examples

**Basic validation:**

```yaml
- name: Validate agent configs
  uses: avifenesh/agnix@v0.1.0
```

**Strict mode with specific target:**

```yaml
- name: Validate Claude Code configs
  uses: avifenesh/agnix@v0.1.0
  with:
    target: 'claude-code'
    strict: 'true'
```

**With SARIF upload to GitHub Code Scanning:**

```yaml
- name: Validate agent configs
  id: agnix
  uses: avifenesh/agnix@v0.1.0
  with:
    format: 'sarif'

- name: Upload SARIF results
  uses: github/codeql-action/upload-sarif@v3
  with:
    sarif_file: ${{ steps.agnix.outputs.sarif-file }}
```

**Conditional failure based on outputs:**

```yaml
- name: Validate agent configs
  id: validate
  uses: avifenesh/agnix@v0.1.0
  with:
    fail-on-error: 'false'

- name: Check results
  if: steps.validate.outputs.errors > 0
  run: |
    echo "Found ${{ steps.validate.outputs.errors }} errors"
    exit 1
```

## Performance

agnix validates files in parallel using [rayon](https://github.com/rayon-rs/rayon) for optimal performance on large projects. Results are sorted deterministically (errors first, then by file path) to ensure consistent output across runs.

## Quality Assurance

This project uses comprehensive CI to ensure code quality:

- **CI Pipeline** - Format checks, clippy linting, unused dependency detection, and cross-platform testing (Linux, macOS, Windows with stable and beta Rust)
- **Security Scanning** - CodeQL static analysis and cargo-audit for vulnerability detection
- **Changelog Validation** - PRs must update CHANGELOG.md (skip with `[skip changelog]` in PR title)

## Configuration

Create `.agnix.toml` in your project:

```toml
severity = "Warning"
target = "Generic"  # Options: Generic, ClaudeCode, Cursor, Codex

[rules]
# Category toggles - enable/disable entire rule categories
skills = true       # AS-*, CC-SK-* rules
hooks = true        # CC-HK-* rules
agents = true       # CC-AG-* rules
copilot = true        # COP-* rules
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

| Category | Rules | Description |
|----------|-------|-------------|
| skills | AS-*, CC-SK-* | Agent skill validation |
| hooks | CC-HK-* | Hook configuration validation |
| agents | CC-AG-* | Subagent validation |
| copilot | COP-* | GitHub Copilot instruction validation |
| memory | CC-MEM-* | Memory/CLAUDE.md validation |
| plugins | CC-PL-* | Plugin validation |
| mcp | MCP-* | MCP tool validation |
| prompt_engineering | PE-* | Prompt engineering best practices |
| xml | XML-* | XML tag balance |
| imports | imports::* | Import reference validation |

## Supported Standards

- **Agent Skills** - [agentskills.io](https://agentskills.io) open standard
- **MCP** - [Model Context Protocol](https://modelcontextprotocol.io)
- **Claude Code** - Hooks, Memory, Plugins, Subagents
- **A2A** - Agent-to-Agent protocol (coming soon)

## Development

```bash
# Build
cargo build

# Run tests
cargo test

# Run CLI
cargo run --bin agnix -- .

# Watch mode
cargo watch -x test
```

### Releasing

To create a release, push a version tag:

```bash
git tag v0.1.0
git push origin v0.1.0
```

This triggers the release workflow which:
- Builds binaries for Linux (gnu/musl), macOS (x86/ARM), and Windows
- Creates archives with SHA256 checksums
- Extracts release notes from CHANGELOG.md
- Publishes to GitHub Releases

Pre-release versions (e.g., `v0.1.0-beta`) are marked as pre-release automatically.

## Project Structure

```
agnix/
├── crates/
│   ├── agnix-core/        # Core validation engine
│   ├── agnix-cli/         # CLI binary
│   ├── agnix-lsp/         # LSP server (coming)
│   └── agnix-wasm/        # WASM for VS Code (coming)
├── tests/
│   └── fixtures/          # Test configs
└── editors/
    └── vscode/            # VS Code extension (coming)
```

## Roadmap

- [x] Core validation engine
- [x] CLI with colored output
- [x] Agent Skills validation (AS-* + CC-SK-001 to CC-SK-009)
- [x] CLAUDE.md rules
- [x] XML balance checking
- [x] @import resolution
- [x] Hooks validation (CC-HK-001 to CC-HK-011)
- [x] Agent validation (CC-AG-001 to CC-AG-006)
- [x] Parallel file validation
- [x] Config-based rule filtering
- [x] Auto-fix infrastructure (--fix, --dry-run, --fix-safe)
- [x] Plugin validation (CC-PL-001 to CC-PL-005)
- [x] MCP tool validation (MCP-001 to MCP-006)
- [x] GitHub Action for CI/CD integration
- [ ] LSP server
- [ ] VS Code extension

## License

MIT OR Apache-2.0

## Author

Avi Fenesh - [@avifenesh](https://github.com/avifenesh)
