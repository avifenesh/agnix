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

With nvim-lspconfig:

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

agnix provides a native JetBrains plugin for IntelliJ IDEA, WebStorm, PyCharm, and all other JetBrains IDEs.

### Installation

#### From JetBrains Marketplace

1. Open **Settings/Preferences** > **Plugins**
2. Search for "agnix"
3. Click **Install**
4. Restart your IDE

#### Manual Installation

1. Download the latest release from [GitHub Releases](https://github.com/avifenesh/agnix/releases)
2. Open **Settings/Preferences** > **Plugins**
3. Click the gear icon > **Install Plugin from Disk...**
4. Select the downloaded `.zip` file
5. Restart your IDE

### Requirements

- JetBrains IDE 2023.2 or later
- [LSP4IJ](https://plugins.jetbrains.com/plugin/23257-lsp4ij) plugin (installed automatically as dependency)

### Configuration

Open **Settings/Preferences** > **Tools** > **agnix** to configure:

- **Enable**: Toggle validation on/off
- **LSP binary path**: Custom path to agnix-lsp (leave empty for auto-detection)
- **Auto-download**: Automatically download LSP binary if not found
- **CodeLens**: Show CodeLens annotations
- **Trace level**: Debug LSP communication (off, messages, verbose)

### Usage

1. Open any supported file (e.g., `SKILL.md`, `CLAUDE.md`)
2. Issues appear automatically in the **Problems** panel
3. Hover over highlighted text for details
4. Use quick fixes (lightbulb icon or Alt+Enter) to resolve issues

### Context Menu

Right-click in the editor to access:
- **agnix** > **Validate Current File**
- **agnix** > **Restart Language Server**
- **agnix** > **Settings**

### Troubleshooting

**Language server not starting:**

1. Check **Settings** > **Tools** > **agnix** for correct configuration
2. Verify agnix-lsp binary exists at the configured path
3. Try **Tools** > **agnix** > **Restart Language Server**
4. Check the IDE log for errors (**Help** > **Show Log in Explorer/Finder**)

**Binary not found:**

The plugin can automatically download the LSP binary:
1. Enable **Auto-download** in settings
2. Or manually install: `cargo install agnix-lsp`
3. Or download from [GitHub Releases](https://github.com/avifenesh/agnix/releases)

## Supported File Types

- `SKILL.md` - Agent skill definitions
- `CLAUDE.md`, `AGENTS.md` - Memory files
- `.claude/settings.json` - Hook configurations
- `plugin.json` - Plugin manifests
- `*.mcp.json` - MCP tool configurations
- `.github/copilot-instructions.md` - Copilot instructions
- `.cursor/rules/*.mdc` - Cursor project rules

## Features

- Real-time diagnostics as you type
- Quick-fix code actions for auto-fixable issues
- Hover documentation for frontmatter fields
- 100 validation rules
- Status bar indicator (VS Code)
- Syntax highlighting for SKILL.md (VS Code)
