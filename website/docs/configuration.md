---
title: Configuration
description: "Configure agnix with .agnix.toml -- target tools, disable rules, set output format, and more."
---

# Configuration

agnix works with zero configuration. To customize, add `.agnix.toml` to your project root.

## Example

```toml
target = "claude-code"
strict = false
max_files = 10000
locale = "en"
disabled_rules = []
```

## Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `target` | string | none | Single tool focus: `claude-code`, `cursor`, `codex`, `copilot` |
| `tools` | string[] | all | Multi-tool targeting. Overrides `target`. |
| `strict` | bool | `false` | Treat warnings as errors |
| `fix` | bool | `false` | Apply available auto-fixes |
| `max_files` | int | `10000` | Maximum files to scan |
| `locale` | string | `"en"` | Output locale |
| `disabled_rules` | string[] | `[]` | Rule IDs to skip (e.g. `["CC-MEM-005"]`) |
| `format` | string | `"text"` | Output format: `text`, `json`, `sarif` |

## CLI flags

CLI flags override `.agnix.toml` values:

```bash
# Target a specific tool
agnix --target cursor .

# Apply fixes
agnix --fix .

# JSON output for CI
agnix --format json .

# SARIF output for GitHub Code Scanning
agnix --format sarif .

# Strict mode
agnix --strict .
```

## Full reference

For the complete configuration specification, see
[docs/CONFIGURATION.md](https://github.com/avifenesh/agnix/blob/main/docs/CONFIGURATION.md)
in the repository.
