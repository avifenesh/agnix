<div align="center">
  <img src="editors/vscode/icon.png" alt="agnix" width="128">
  <h1>agnix</h1>
  <p><strong>Lint agent configurations before they break your workflow</strong></p>
  <p>
    <a href="https://www.npmjs.com/package/agnix"><img src="https://img.shields.io/npm/v/agnix.svg" alt="npm"></a>
    <a href="https://crates.io/crates/agnix-cli"><img src="https://img.shields.io/crates/v/agnix-cli.svg" alt="Crates.io"></a>
    <a href="https://github.com/avifenesh/agnix/releases"><img src="https://img.shields.io/github/v/release/avifenesh/agnix" alt="Release"></a>
    <a href="https://github.com/avifenesh/agnix/actions/workflows/ci.yml"><img src="https://github.com/avifenesh/agnix/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
    <a href="LICENSE-MIT"><img src="https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg" alt="License"></a>
  </p>
</div>

Validates AI agent configs across Claude Code, Cursor, GitHub Copilot, Codex CLI, and more.

**100 rules** | **1500+ tests** | **Parallel validation** | **LSP server** | **MCP server**

## Why agnix?

The AI coding landscape is chaos. Every tool wants your config in a different format:

| Tool | Config File | Format |
|------|-------------|--------|
| Claude Code | `CLAUDE.md`, `.claude/settings.json` | Markdown + JSON |
| Cursor | `.cursor/rules/*.mdc` | MDC |
| GitHub Copilot | `.github/copilot-instructions.md` | Markdown |
| Codex CLI | `AGENTS.md` | Markdown |
| MCP | `*.mcp.json` | JSON Schema |

**The problems are real:**

- **Skills don't auto-trigger** - Vercel's research found [skills invoke at 0%](https://vercel.com/blog/agents-md-outperforms-skills-in-our-agent-evals) without explicit prompting. Wrong syntax means your skill never runs.
- **Almost-right is worse than wrong** - [66% of developers](https://survey.stackoverflow.co/2025/ai) cite "AI solutions that are almost right" as their biggest frustration. Broken configs cause exactly this.
- **Unbundled stack, fragmented configs** - Developers mix Cursor + Claude Code + Copilot. A config that works in one tool [silently fails in another](https://arnav.tech/beyond-copilot-cursor-and-claude-code-the-unbundled-coding-ai-tools-stack).
- **Inconsistent patterns become chaos amplifiers** - When your config follows wrong patterns, [AI assistants amplify the mistakes](https://www.augmentcode.com/guides/enterprise-coding-standards-12-rules-for-ai-ready-teams), not just ignore them.

agnix validates configs against 100 rules derived from official specs, research papers, and real-world testing. Catch issues before they reach your IDE.

## Features

**Validation**: Skills, Hooks, Agents, Plugins, MCP, Memory, Prompt Engineering, XML, Imports, Cross-platform

**Tools**: Claude Code, Cursor, GitHub Copilot, Codex CLI, AGENTS.md

**Integration**: LSP server, VS Code extension, GitHub Action, auto-fix (`--fix`)

## Installation

```bash
# npm (easiest - all platforms)
npm install -g agnix

# Homebrew (macOS/Linux)
brew tap avifenesh/agnix
brew install agnix

# Cargo (all platforms)
cargo install agnix-cli

# Pre-built binaries
# Download from https://github.com/avifenesh/agnix/releases
```

## Claude Code Skill

Use `/agnix` directly in Claude Code via [awesome-slash](https://github.com/avifenesh/awesome-slash) (300+ stars):

```bash
# Claude Code
/plugin marketplace add avifenesh/awesome-slash
/plugin install agnix@awesome-slash

# Or via npm (all platforms)
npm install -g awesome-slash && awesome-slash
```

Then run `/agnix` to validate your project, `/agnix --fix` to auto-fix issues.

## Quick Start

```bash
# Run without installing
npx agnix .

# Validate current directory
agnix .

# Apply automatic fixes
agnix --fix .

# Strict mode (warnings = errors)
agnix --strict .

# Target specific tool
agnix --target claude-code .

# Telemetry (opt-in usage analytics)
agnix telemetry status   # Check status
agnix telemetry enable   # Enable (opt-in)
agnix telemetry disable  # Disable
```

See [Configuration Reference](docs/CONFIGURATION.md) for all options.

## Output

```
Validating: .

CLAUDE.md:15:1 warning: Generic instruction 'Be helpful and accurate' [fixable]
  help: Remove generic instructions. Claude already knows this.

.claude/skills/review/SKILL.md:3:1 error: Invalid name 'Review-Code' [fixable]
  help: Use lowercase letters and hyphens only (e.g., 'code-review')

Found 1 error, 1 warning
  2 issues are automatically fixable

hint: Run with --fix to apply fixes
```

**Formats**: `--format json` | `--format sarif` (GitHub Code Scanning)

## GitHub Action

```yaml
- name: Validate agent configs
  uses: avifenesh/agnix@v0
  with:
    target: 'claude-code'
```

With SARIF upload to GitHub Code Scanning:

```yaml
- name: Validate agent configs
  id: agnix
  uses: avifenesh/agnix@v0
  with:
    format: 'sarif'

- name: Upload SARIF results
  uses: github/codeql-action/upload-sarif@v3
  with:
    sarif_file: ${{ steps.agnix.outputs.sarif-file }}
```

See [full action documentation](docs/CONFIGURATION.md#github-action) for all inputs and outputs.

## Pre-commit Hook

```yaml
# .pre-commit-config.yaml
repos:
  - repo: https://github.com/avifenesh/agnix
    rev: v0.7.0
    hooks:
      - id: agnix
```

## Editor Integration

```bash
cargo install agnix-lsp
```

Real-time diagnostics as you type, quick-fix code actions, hover documentation.

![VS Code validation example](assets/vscode-validation.png)

See [Editor Setup](docs/EDITOR-SETUP.md) for VS Code, Neovim, Helix configuration.

## MCP Server

Expose agnix validation as MCP tools for AI assistants:

```bash
cargo install agnix-mcp
```

**Tools available:**
- `validate_file` - Validate a single config file
- `validate_project` - Validate all configs in a directory
- `get_rules` - List all 100 validation rules
- `get_rule_docs` - Get details about a specific rule

**Claude Desktop configuration:**

```json
{
  "mcpServers": {
    "agnix": {
      "command": "agnix-mcp"
    }
  }
}
```

The server follows MCP best practices with rich parameter schemas and structured JSON output.

## Configuration

```toml
# .agnix.toml
target = "ClaudeCode"

[rules]
disabled_rules = ["PE-003"]
```

See [Configuration Reference](docs/CONFIGURATION.md) for full options.

## Performance

Benchmarks (run with `cargo bench`):

| Metric | Result |
|--------|--------|
| File type detection | ~85ns/file |
| Single file validation | ~15-50μs |
| Project validation | 5,000+ files/second |
| Registry caching | 7x speedup |

## Supported Tools

| Tool | Rules | Config Files |
|------|-------|--------------|
| [Agent Skills](https://agentskills.io) | AS-*, CC-SK-* | SKILL.md |
| [Claude Code](https://docs.anthropic.com/en/docs/build-with-claude/claude-code) | CC-* | CLAUDE.md, hooks, agents, plugins |
| [GitHub Copilot](https://docs.github.com/en/copilot) | COP-* | .github/copilot-instructions.md |
| [Cursor](https://cursor.com) | CUR-* | .cursor/rules/*.mdc |
| [MCP](https://modelcontextprotocol.io) | MCP-* | *.mcp.json |
| [AGENTS.md](https://agentsmd.org) | AGM-*, XP-* | AGENTS.md |

## Development

```bash
cargo build       # Build
cargo test        # Run tests
cargo run --bin agnix -- .  # Run CLI
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for development guidelines.

## Project Structure

```
crates/
  agnix-core/     # Validation engine (1500+ tests)
  agnix-cli/      # CLI binary
  agnix-lsp/      # Language server
  agnix-mcp/      # MCP server
  agnix-rules/    # Rule metadata
editors/
  vscode/         # VS Code extension
knowledge-base/   # 100 rules documentation
```

## What's Included

- **100 validation rules** across 12 categories
- **1500+ tests** ensuring reliability
- **CLI** with colored output, JSON/SARIF formats
- **LSP server** for real-time editor diagnostics
- **MCP server** for AI assistant integration
- **VS Code extension** with syntax highlighting
- **GitHub Action** for CI/CD integration
- **Auto-fix** infrastructure (--fix, --dry-run, --fix-safe)
- **Parallel validation** using rayon
- **Cross-platform** support (Linux, macOS, Windows)

## Roadmap

See [GitHub Issues](https://github.com/avifenesh/agnix/issues) for the full roadmap.

**Editor integrations**: Neovim plugin, JetBrains IDE, Zed extension

**Features**: Documentation website, additional rule categories

## License

MIT OR Apache-2.0

## Author

Avi Fenesh - [@avifenesh](https://github.com/avifenesh)
