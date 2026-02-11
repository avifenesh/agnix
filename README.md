<div align="center">
  <img src="editors/vscode/icon.png" alt="agnix" width="128">
  <h1>agnix</h1>
  <p><strong>Lint agent configurations before they break your workflow</strong></p>
  <p>
    <a href="https://www.npmjs.com/package/agnix"><img src="https://img.shields.io/npm/v/agnix.svg" alt="npm"></a>
    <a href="https://crates.io/crates/agnix-cli"><img src="https://img.shields.io/crates/v/agnix-cli.svg" alt="Crates.io"></a>
    <a href="https://github.com/avifenesh/agnix/releases"><img src="https://img.shields.io/github/v/release/avifenesh/agnix" alt="Release"></a>
    <a href="https://github.com/avifenesh/agnix/actions/workflows/ci.yml"><img src="https://github.com/avifenesh/agnix/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
    <a href="https://codecov.io/gh/avifenesh/agnix"><img src="https://codecov.io/gh/avifenesh/agnix/branch/main/graph/badge.svg" alt="Coverage"></a>
    <a href="LICENSE-MIT"><img src="https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg" alt="License"></a>
  </p>
</div>

The linter for your AI coding stack -- skills, hooks, memory, plugins, MCP, and agent configs. CLI, LSP server, and IDE plugins for Claude Code, Cursor, GitHub Copilot, Codex CLI, and more.

**156 validation rules** | **Auto-fix** | **[VS Code](https://marketplace.visualstudio.com/items?itemName=avifenesh.agnix) + [JetBrains](https://plugins.jetbrains.com/plugin/30087-agnix) + Neovim + Zed** | **GitHub Action**

<p align="center">
  <a href="https://avifenesh.github.io/agnix/"><img src="https://img.shields.io/badge/Docs-Website-0A7E8C?style=for-the-badge" alt="Website"></a>
  <a href="https://marketplace.visualstudio.com/items?itemName=avifenesh.agnix"><img src="https://img.shields.io/badge/VS%20Code-Install-007ACC?style=for-the-badge" alt="Install VS Code Extension"></a>
  <a href="https://plugins.jetbrains.com/plugin/30087-agnix"><img src="https://img.shields.io/badge/JetBrains-Install-000000?style=for-the-badge" alt="Install JetBrains Plugin"></a>
</p>

## Why agnix?

**Your skills don't trigger.** Vercel's research found skills [invoke at 0%](https://vercel.com/blog/agents-md-outperforms-skills-in-our-agent-evals) without correct syntax. One wrong field and your skill is invisible.

**"Almost right" is the worst outcome.** [66% of developers](https://survey.stackoverflow.co/2025/ai) cite it as their biggest AI frustration. Misconfigured agents produce exactly this.

**Multi-tool stacks fail silently.** Cursor + Claude Code + Copilot each want different formats. A config that works in one tool [breaks in another](https://arnav.tech/beyond-copilot-cursor-and-claude-code-the-unbundled-coding-ai-tools-stack).

**Bad patterns get amplified.** AI assistants don't ignore wrong configs -- they [learn from them](https://www.augmentcode.com/guides/enterprise-coding-standards-12-rules-for-ai-ready-teams).

agnix catches all of this. 156 rules derived from official specs, research papers, and real-world testing. Auto-fix included.

## Quick Start

```console
$ npx agnix .
Validating: .

CLAUDE.md:15:1 warning: Generic instruction 'Be helpful and accurate' [fixable]
  help: Remove generic instructions. Claude already knows this.

.claude/skills/review/SKILL.md:3:1 error: Invalid name 'Review-Code' [fixable]
  help: Use lowercase letters and hyphens only (e.g., 'code-review')

Found 1 error, 1 warning
  2 issues are automatically fixable

hint: Run with --fix to apply fixes
```

<p align="center">
  <img src="assets/demo.gif" alt="agnix real-time validation in VS Code" width="720">
</p>

## Install

```bash
# npm (recommended, all platforms)
npm install -g agnix

# Homebrew (macOS/Linux)
brew tap avifenesh/agnix && brew install agnix

# Cargo
cargo install agnix-cli
```

[Pre-built binaries](https://github.com/avifenesh/agnix/releases) | [All install options](https://avifenesh.github.io/agnix/docs/installation)

### Editor Extensions

| Editor | Install |
|--------|---------|
| **VS Code** | [VS Code Marketplace](https://marketplace.visualstudio.com/items?itemName=avifenesh.agnix) |
| **JetBrains** | [JetBrains Marketplace](https://plugins.jetbrains.com/plugin/30087-agnix) |
| **Neovim** | `{ "avifenesh/agnix.nvim" }` |
| **Zed** | Search "agnix" in Extensions |

[Editor setup guide](https://avifenesh.github.io/agnix/docs/editor-integration)

### GitHub Action

```yaml
- name: Validate agent configs
  uses: avifenesh/agnix@v0
  with:
    target: 'claude-code'
```

## Usage

```bash
agnix .              # Validate current directory
agnix --fix .        # Apply automatic fixes
agnix --fix-safe .   # Apply only safe fixes
agnix --strict .     # Strict mode (warnings = errors)
agnix --target claude-code .  # Target specific tool
```

[Full CLI reference](https://avifenesh.github.io/agnix/docs/configuration) | [All 156 rules](https://avifenesh.github.io/agnix/docs/rules)

## Supported Tools

| Tool | Rules | Config Files |
|------|-------|--------------|
| [Agent Skills](https://agentskills.io) | AS-\*, CC-SK-\* | SKILL.md |
| [Claude Code](https://docs.anthropic.com/en/docs/build-with-claude/claude-code) | CC-\* | CLAUDE.md, hooks, agents, plugins |
| [GitHub Copilot](https://docs.github.com/en/copilot) | COP-\* | .github/copilot-instructions.md, .github/instructions/\*.instructions.md |
| [Cursor](https://cursor.com) | CUR-\* | .cursor/rules/\*.mdc, .cursorrules |
| [MCP](https://modelcontextprotocol.io) | MCP-\* | \*.mcp.json |
| [AGENTS.md](https://agentsmd.org) | AGM-\*, XP-\* | AGENTS.md, AGENTS.local.md, AGENTS.override.md |
| [Gemini CLI](https://github.com/google-gemini/gemini-cli) | GM-\* | GEMINI.md, GEMINI.local.md |

## Contributing

Contributions welcome. See [CONTRIBUTING.md](CONTRIBUTING.md) for the development guide.

[Report a bug](https://github.com/avifenesh/agnix/issues/new) | [Request a rule](https://github.com/avifenesh/agnix/issues/new) | [Good first issues](https://github.com/avifenesh/agnix/labels/good%20first%20issue)

## License

MIT OR Apache-2.0

---

<p align="center">
  <a href="https://github.com/avifenesh/agnix/stargazers">Star this repo</a> to help other developers find agnix.
</p>
