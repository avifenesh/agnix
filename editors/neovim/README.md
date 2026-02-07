# agnix.nvim

Neovim plugin for [agnix](https://github.com/avifenesh/agnix) - lint agent configurations before they break your workflow.

Provides real-time validation of AI agent configuration files (CLAUDE.md, AGENTS.md, SKILL.md, `.claude/settings.json`, `*.mcp.json`, `.cursor/rules/*.mdc`, and more) using the `agnix-lsp` language server.

## Features

- Automatic LSP attachment to supported file types
- Real-time diagnostics as you type
- Quick-fix code actions for auto-fixable issues
- Hover documentation for configuration fields
- 139 validation rules across 17 categories
- Commands for server management and rule browsing
- Optional Telescope integration
- `:checkhealth` support

## Requirements

- Neovim >= 0.9
- `agnix-lsp` binary

Install the LSP server:

```bash
# npm (easiest)
npm install -g agnix

# Cargo
cargo install agnix-lsp

# Or download from releases
# https://github.com/avifenesh/agnix/releases
```

## Installation

### lazy.nvim

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

### packer.nvim

```lua
use {
  'avifenesh/agnix',
  config = function()
    require('agnix').setup()
  end,
}
```

### vim-plug

```vim
Plug 'avifenesh/agnix'
```

Then in your `init.lua`:

```lua
require('agnix').setup()
```

### Manual

Copy the `editors/neovim/` directory contents to:

```
~/.local/share/nvim/site/pack/plugins/start/agnix/
```

## Configuration

```lua
require('agnix').setup({
  -- Path to agnix-lsp binary (nil = auto-detect)
  cmd = nil,

  -- Neovim filetypes that may contain agnix files
  filetypes = { 'markdown', 'json' },

  -- Markers for project root detection
  root_markers = { '.git', '.agnix.toml', 'CLAUDE.md', 'AGENTS.md' },

  -- Start LSP automatically when opening a matching file
  autostart = true,

  -- Callback when LSP attaches to a buffer
  on_attach = function(client, bufnr)
    -- Add buffer-local keymaps, etc.
  end,

  -- LSP settings sent to the server
  settings = {
    -- Minimum severity: 'Error', 'Warning', 'Info'
    severity = nil,

    -- Target tool: 'Generic', 'ClaudeCode', 'Cursor', 'Codex'
    target = nil,

    -- Tools to validate for
    tools = nil,

    -- Rule category toggles
    rules = {
      skills = nil,           -- AS-*, CC-SK-*
      hooks = nil,            -- CC-HK-*
      agents = nil,           -- CC-AG-*
      memory = nil,           -- CC-MEM-*
      plugins = nil,          -- CC-PL-*
      xml = nil,              -- XML-*
      mcp = nil,              -- MCP-*
      imports = nil,          -- REF-*
      cross_platform = nil,   -- XP-*
      agents_md = nil,        -- AGM-*
      copilot = nil,          -- COP-*
      cursor = nil,           -- CUR-*
      prompt_engineering = nil, -- PE-*
      disabled_rules = nil,   -- List of rule IDs to disable
    },

    -- Tool version pins
    versions = {
      claude_code = nil,
      codex = nil,
      cursor = nil,
      copilot = nil,
    },

    -- Specification revision pins
    specs = {
      mcp_protocol = nil,
      agent_skills_spec = nil,
      agents_md_spec = nil,
    },
  },

  -- Log level: 'trace', 'debug', 'info', 'warn', 'error'
  log_level = 'warn',

  -- Telescope integration
  telescope = { enable = true },
})
```

## Commands

| Command | Description |
|---------|-------------|
| `:AgnixStart` | Start the LSP server |
| `:AgnixStop` | Stop the LSP server |
| `:AgnixRestart` | Restart the LSP server |
| `:AgnixInfo` | Show server status and info |
| `:AgnixValidateFile` | Trigger re-validation of the current file |
| `:AgnixShowRules` | Browse rule categories (Telescope or fallback) |
| `:AgnixFixAll` | Apply all code action fixes |
| `:AgnixFixSafe` | Apply only preferred (safe) fixes |
| `:AgnixIgnoreRule {id}` | Disable a rule in `.agnix.toml` |
| `:AgnixShowRuleDoc {id}` | Open rule documentation in the browser |

## Telescope Integration

If [telescope.nvim](https://github.com/nvim-telescope/telescope.nvim) is installed:

```lua
require('telescope').load_extension('agnix')
```

Then use:
- `:Telescope agnix rules` -- Browse rule categories
- `:Telescope agnix diagnostics` -- View agnix diagnostics

## Supported File Types

| File Pattern | Type |
|---|---|
| `SKILL.md` | Agent skill definitions |
| `CLAUDE.md`, `CLAUDE.local.md` | Claude Code memory |
| `AGENTS.md`, `AGENTS.local.md`, `AGENTS.override.md` | Agent memory |
| `.claude/settings.json`, `.claude/settings.local.json` | Hook configurations |
| `plugin.json` | Plugin manifests |
| `*.mcp.json`, `mcp.json`, `mcp-*.json` | MCP tool configurations |
| `.github/copilot-instructions.md` | Copilot instructions |
| `.github/instructions/*.instructions.md` | Copilot scoped instructions |
| `.cursor/rules/*.mdc` | Cursor project rules |
| `.cursorrules` | Legacy Cursor rules |
| `.claude/agents/*.md` | Claude agent definitions |

## Health Check

Run `:checkhealth agnix` to verify your setup.

## Troubleshooting

**agnix-lsp not found**

```bash
# Check if installed
which agnix-lsp   # Unix
where agnix-lsp   # Windows

# Or set the path explicitly
require('agnix').setup({ cmd = '/path/to/agnix-lsp' })
```

**No diagnostics appearing**

1. Verify the file is a supported type (see table above)
2. Run `:AgnixInfo` to check server status
3. Run `:checkhealth agnix` for detailed diagnostics
4. Check `:LspLog` for server errors

**Manual LSP alternative (without this plugin)**

If you prefer using nvim-lspconfig directly:

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

## License

MIT - see [LICENSE](LICENSE)
