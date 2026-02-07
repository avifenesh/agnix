---
title: Getting Started
description: "Install agnix and validate your agent configuration files in under 60 seconds."
---

# Getting Started

## 1. Run agnix

No installation needed. Use `npx` to run against your project:

```bash
npx agnix .
```

Expected output:

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

## 2. Auto-fix issues

```bash
npx agnix --fix .
```

agnix applies safe fixes automatically and reports what changed.

## 3. Install globally (optional)

If you use agnix regularly:

```bash
npm install -g agnix
```

Then run with:

```bash
agnix .
```

See [Installation](./installation.md) for Homebrew, Cargo, and binary options.

## 4. Target a specific tool

Validate only configs relevant to a single tool:

```bash
agnix --target claude-code .
agnix --target cursor .
agnix --target copilot .
```

## Next steps

- [Configuration](./configuration.md) -- customize rules with `.agnix.toml`
- [Rules Reference](./rules/index.md) -- browse all 139 rules
- [Editor Integration](./editor-integration.md) -- get diagnostics in your editor
- [Troubleshooting](./troubleshooting.md) -- common issues and fixes
