# agnix

> The nginx of agent configs

Validate agent specifications across Claude Code, Cursor, Codex, and beyond.

**Validates:** Skills • MCP • Hooks • Memory • Plugins

```bash
agnix .
```

## Features

- ✅ **Agent Skills** - Validates SKILL.md format (agentskills.io spec)
- ✅ **Claude Code** - CLAUDE.md, hooks, subagents, plugins
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

# Generate config file
agnix init
```

## Output

```
Validating: .

CLAUDE.md:15:1 warning: Generic instruction 'Be helpful and accurate'
  help: Remove generic instructions. Claude already knows this.

.claude/skills/review/SKILL.md:3:1 error: Invalid name 'Review-Code'
  help: Use lowercase letters and hyphens only (e.g., 'code-review')

.claude/skills/review/SKILL.md:4:8 error: Unknown model 'gpt-4'
  help: Use: sonnet, opus, haiku, inherit

────────────────────────────────────────────────────────────
Found 2 errors, 1 warning
```

## Performance

agnix validates files in parallel using [rayon](https://github.com/rayon-rs/rayon) for optimal performance on large projects. Results are sorted deterministically (errors first, then by file path) to ensure consistent output across runs.

## Configuration

Create `.agnix.toml` in your project:

```toml
severity = "Warning"
target = "Generic"

[rules]
generic_instructions = true
frontmatter_validation = true
xml_balance = true
import_references = true
tool_names = true
required_fields = true

[[exclude]]
"node_modules/**"
".git/**"
"target/**"
```

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
- [x] Hooks validation (CC-HK-006 to CC-HK-009)
- [x] Parallel file validation
- [ ] MCP tool validation
- [ ] LSP server
- [ ] VS Code extension
- [ ] Auto-fix mode

## License

MIT OR Apache-2.0

## Author

Avi Fenesh - [@avifenesh](https://github.com/avifenesh)
