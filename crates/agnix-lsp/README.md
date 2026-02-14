# agnix-lsp

Language Server Protocol implementation for agnix.

Provides real-time validation of agent configuration files in editors that support LSP.

## Installation

```bash
cargo install agnix-lsp
```

Or build from the workspace root:

```bash
cargo build --release -p agnix-lsp
```

The binary will be at `target/release/agnix-lsp`.

## Usage

The server communicates over stdin/stdout using the LSP protocol:

```bash
agnix-lsp
```

## Editor Configuration

### VS Code

A dedicated VS Code extension is available at `editors/vscode`. See `editors/vscode/README.md` for installation and usage.

### Neovim (with nvim-lspconfig)

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

### Helix

Add to `~/.config/helix/languages.toml`:

```toml
[[language]]
name = "markdown"
language-servers = ["agnix-lsp"]

[language-server.agnix-lsp]
command = "agnix-lsp"
```

### Zed

Install the agnix extension from the Zed Extensions panel:

1. Open Zed > Extensions (`cmd+shift+x`)
2. Search for "agnix"
3. Click Install

The extension automatically downloads the `agnix-lsp` binary. See `editors/zed/README.md` for details.

## Features

- Real-time diagnostics as you type (via textDocument/didChange)
- Real-time diagnostics on file open and save
- Supports all agnix validation rules (224 rules)
- Project-level validation for cross-file rules (AGM-006, XP-004/005/006, VER-001)

- Maps diagnostic severity levels (Error, Warning, Info)
- Rule codes shown in diagnostic messages
- Quick-fix code actions for auto-fixable diagnostics
- Hover documentation for frontmatter fields (name, version, model, etc.)
- Context-aware completions for frontmatter keys, values, and snippets

## Supported File Types

The LSP server validates the same file types as the CLI:

- `SKILL.md` - Agent skill definitions
- `CLAUDE.md`, `CLAUDE.local.md`, `AGENTS.md`, `AGENTS.local.md`, `AGENTS.override.md` - Memory/instruction files
- `.claude/settings.json`, `.claude/settings.local.json` - Hook configurations
- `plugin.json` - Plugin manifests
- `*.mcp.json`, `mcp.json`, `mcp-*.json` - MCP tool configurations
- `.github/copilot-instructions.md`, `.github/instructions/*.instructions.md`, `.github/agents/*.agent.md`, `.github/prompts/*.prompt.md`, `.github/hooks/hooks.json`, `.github/workflows/copilot-setup-steps.yml` - GitHub Copilot configuration
- `.cursor/rules/*.mdc`, `.cursorrules` - Cursor project rules

## Development

Run tests:

```bash
cargo test -p agnix-lsp
```

Build in debug mode:

```bash
cargo build -p agnix-lsp
```

## Architecture

```
agnix-lsp/
├── src/
│   ├── lib.rs              # Public API and server setup
│   ├── main.rs             # Binary entry point
│   ├── backend.rs          # LSP backend facade and LanguageServer wiring
│   ├── backend/
│   │   ├── events.rs       # did_open/did_change/did_save/did_close handlers
│   │   ├── helpers.rs      # Diagnostics and path normalization helpers
│   │   ├── revalidation.rs # Config and project revalidation orchestration
│   │   └── tests.rs        # Backend unit and regression tests
│   ├── diagnostic_mapper.rs # Converts agnix diagnostics to LSP format
│   ├── code_actions.rs      # Quick-fix code action generation
│   ├── completion_provider.rs # Context-aware frontmatter completions
│   ├── hover_provider.rs    # Hover documentation for frontmatter fields
│   ├── locale.rs            # Localization support
│   ├── position.rs          # Byte offset to LSP position conversion
│   └── vscode_config.rs     # VS Code settings integration
└── tests/
    └── lsp_integration.rs  # Integration tests
```

## License

MIT OR Apache-2.0
