# agnix

> The nginx of agent configs

Validate agent specifications across Claude Code, Cursor, Codex, and beyond.

**Validates:** Skills • MCP • Hooks • Memory • Agents • Plugins

```bash
agnix .
```

## Features

- ✅ **Agent Skills** - Validates SKILL.md format (agentskills.io spec)
- ✅ **Claude Code** - CLAUDE.md, hooks, subagents, plugins
- ✅ **Subagents** - Agent frontmatter validation (CC-AG-001 to CC-AG-006)
- ✅ **Plugins** - Plugin manifest validation (CC-PL-001 to CC-PL-005)
- ✅ **Generic Instructions** - Detects redundant "be helpful" patterns
- ✅ **XML Balance** - Ensures tags are properly closed
- ✅ **@imports** - Validates file references exist
- ✅ **Hooks** - Event and config validation (CC-HK-006 to CC-HK-009)
- ✅ **Parallel Validation** - Fast processing of large projects using rayon
- 🚧 **MCP Tools** - Schema validation (coming soon)
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

# Generate config file
agnix init
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

| Category | Rules | Description |
|----------|-------|-------------|
| skills | AS-*, CC-SK-* | Agent skill validation |
| hooks | CC-HK-* | Hook configuration validation |
| agents | CC-AG-* | Subagent validation |
| memory | CC-MEM-* | Memory/CLAUDE.md validation |
| plugins | CC-PL-* | Plugin validation |
| xml | xml::* | XML tag balance |
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
- [x] CLI with miette errors
- [x] Agent Skills validation
- [x] CLAUDE.md rules
- [x] XML balance checking
- [x] @import resolution
- [x] Hooks validation (CC-HK-001 to CC-HK-009)
- [x] Agent validation (CC-AG-001 to CC-AG-006)
- [x] Parallel file validation
- [x] Config-based rule filtering
- [x] Auto-fix infrastructure (--fix, --dry-run, --fix-safe)
- [x] Plugin validation (CC-PL-001 to CC-PL-005)
- [ ] MCP tool validation
- [ ] LSP server
- [ ] VS Code extension

## License

MIT OR Apache-2.0

## Author

Avi Fenesh - [@avifenesh](https://github.com/avifenesh)
