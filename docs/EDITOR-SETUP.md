# Editor Setup

Real-time validation in your editor using the agnix LSP server.

## Installation

```bash
cargo install agnix-lsp
```

Or build from source:

```bash
cargo build --release -p agnix-lsp
# Binary at target/release/agnix-lsp
```

## VS Code

The VS Code extension auto-downloads `agnix-lsp` on first use. Manual install is optional.

Install the extension from source:

```bash
cd editors/vscode
npm install
npm run compile
npm run package
code --install-extension agnix-*.vsix
```

### Settings

```json
{
  "agnix.lspPath": "agnix-lsp",
  "agnix.enable": true,
  "agnix.trace.server": "off"
}
```

### Commands

- `agnix: Restart Language Server` - Restart the server
- `agnix: Show Output Channel` - View debug output

### Troubleshooting

**agnix-lsp not found:**

```bash
# Check if installed
which agnix-lsp  # Unix
where agnix-lsp  # Windows

# Or specify full path in settings
"agnix.lspPath": "/path/to/agnix-lsp"
```

**No diagnostics appearing:**

1. Check file is a supported type (see below)
2. Verify server is running (check status bar)
3. Open output channel for errors

## Neovim

### agnix.nvim Plugin (Recommended)

The agnix Neovim plugin provides automatic LSP attachment, file type
detection, commands, Telescope integration, and health checks.

With lazy.nvim:

```lua
{
  'avifenesh/agnix',
  ft = { 'markdown', 'json' },
  opts = {},
  config = function(_, opts)
    require('agnix').setup(opts)
  end,
}
```

With packer.nvim:

```lua
use {
  'avifenesh/agnix',
  config = function()
    require('agnix').setup()
  end,
}
```

See [editors/neovim/README.md](../editors/neovim/README.md) for full
configuration, commands, and troubleshooting.

### Manual Setup with nvim-lspconfig

If you prefer manual configuration without the plugin:

```lua
local lspconfig = require('lspconfig')
local configs = require('lspconfig.configs')

if not configs.agnix then
  configs.agnix = {
    default_config = {
      cmd = { 'agnix-lsp' },
      filetypes = { 'markdown', 'json' },
      root_dir = function(fname)
        return lspconfig.util.find_git_ancestor(fname)
      end,
      settings = {},
    },
  }
end

lspconfig.agnix.setup{}
```

Note: The manual approach attaches to all markdown and JSON files. The
plugin is smarter and only attaches to files that agnix actually validates.

## Helix

Add to `~/.config/helix/languages.toml`:

```toml
[[language]]
name = "markdown"
language-servers = ["agnix-lsp"]

[language-server.agnix-lsp]
command = "agnix-lsp"
```

## Cursor

Cursor is built on VS Code, so the VS Code extension works directly. The extension validates `.cursor/rules/*.mdc` files automatically.

### Installation

1. Install the VS Code extension (see VS Code section above)
2. Cursor will detect and use it automatically

### Cursor-Specific Validation

agnix validates Cursor project rules with the following rules:

| Rule | Severity | Description |
|------|----------|-------------|
| CUR-001 | ERROR | Empty .mdc rule file |
| CUR-002 | WARNING | Missing frontmatter |
| CUR-003 | ERROR | Invalid YAML frontmatter |
| CUR-004 | ERROR | Invalid glob pattern in globs field |
| CUR-005 | WARNING | Unknown frontmatter keys |
| CUR-006 | WARNING | Legacy .cursorrules detected |

### File Structure

Cursor project rules should be in `.cursor/rules/`:

```
.cursor/
  rules/
    typescript.mdc
    testing.mdc
    documentation.mdc
```

### MDC Frontmatter

Each `.mdc` file should have frontmatter:

```markdown
---
description: TypeScript coding standards
globs: ["**/*.ts", "**/*.tsx"]
alwaysApply: false
---

# TypeScript Rules

Your rules here...
```

### Migration from .cursorrules

If using legacy `.cursorrules` file, agnix warns about migration (CUR-006). To migrate:

1. Create `.cursor/rules/` directory
2. Split rules into focused `.mdc` files
3. Add frontmatter with `description` and `globs`
4. Delete `.cursorrules`

## JetBrains IDEs

JetBrains plugin source is in `editors/jetbrains/` and integrates with `agnix-lsp` through LSP4IJ.

### Build and Run

```bash
cd editors/jetbrains
./gradlew test
./gradlew buildPlugin
./gradlew runIde
```

### Settings

- **Enable agnix validation**: Global plugin on/off
- **LSP binary path**: Custom path to `agnix-lsp` (optional)
- **Auto-download**: Automatically install `agnix-lsp` if missing
- **Trace level**: `off`, `messages`, `verbose`
- **CodeLens**: Show inline rule annotations

### Real IDE Validation Checklist

1. Install built zip in IntelliJ IDEA, WebStorm, and PyCharm (2023.3+).
2. Confirm diagnostics on supported files and no diagnostics on unrelated files.
3. Confirm `agnix-lsp` auto-download works when binary is missing.
4. Confirm manual `LSP binary path` override works.
5. Confirm `Restart Language Server` action reconnects cleanly.

## Zed

Zed extension source is in `editors/zed/` and provides automatic LSP binary download.

### Installation

1. Open Zed
2. Open the Extensions panel (Zed > Extensions, or `cmd+shift+x`)
3. Search for "agnix"
4. Click Install

The extension automatically downloads the `agnix-lsp` binary from GitHub releases.

### Configuration

agnix reads configuration from `.agnix.toml` in your project root.

### Troubleshooting

**No diagnostics appearing:**

1. Verify the file is a supported type
2. Check Zed log (`cmd+shift+p` > "zed: open log") for LSP errors
3. Ensure project has a `.git` directory or `.agnix.toml` for root detection

**LSP binary download fails:**

1. Check internet connection
2. Verify access to https://github.com/avifenesh/agnix/releases
3. Try restarting Zed to trigger fresh download

**Manual installation:**

```bash
# npm
npm install -g agnix

# Cargo
cargo install agnix-lsp
```

## Supported File Types

- `SKILL.md` - Agent skill definitions
- `CLAUDE.md`, `CLAUDE.local.md`, `AGENTS.md`, `AGENTS.local.md`, `AGENTS.override.md` - Memory files
- `.claude/settings.json`, `.claude/settings.local.json` - Hook configurations
- `plugin.json` - Plugin manifests
- `*.mcp.json`, `mcp.json` - MCP tool configurations
- `.github/copilot-instructions.md`, `.github/instructions/*.instructions.md` - Copilot instructions
- `.cursor/rules/*.mdc`, `.cursorrules` - Cursor project rules

## Features

- Real-time diagnostics as you type
- Quick-fix code actions for auto-fixable issues
- Hover documentation for frontmatter fields
- 100 validation rules
- Status bar indicator (VS Code)
- Syntax highlighting for SKILL.md (VS Code)
