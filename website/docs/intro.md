---
title: Introduction
slug: /
description: "agnix validates AI agent configuration files across Claude Code, Cursor, Copilot, MCP, and AGENTS.md. 139 rules, auto-fix, and editor integration."
---

# agnix

agnix is a linter for AI agent configuration files. It validates Skills, Hooks, Memory, Plugins, MCP configs, and more across Claude Code, Cursor, GitHub Copilot, Codex CLI, and other tools.

```bash
npx agnix .
```

## What it does

- **Validates** configuration files against 139 rules derived from official specs and real-world testing
- **Auto-fixes** common issues with `--fix`
- **Integrates** with VS Code, Neovim, JetBrains, and Zed via the LSP server
- **Outputs** in text, JSON, or SARIF for CI integration

## Next steps

- [Getting Started](./getting-started.md) -- install and run in 60 seconds
- [Rules Reference](./rules/index.md) -- browse all 139 validation rules
- [Configuration](./configuration.md) -- customize with `.agnix.toml`
- [Editor Integration](./editor-integration.md) -- set up real-time diagnostics
