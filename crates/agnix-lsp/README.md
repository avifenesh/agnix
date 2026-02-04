# agnix-lsp

Language Server Protocol implementation for agnix.

Provides real-time validation of agent configuration files in editors that support LSP.

## Installation

```bash
cargo install --path crates/agnix-lsp
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

## Features

- Real-time diagnostics on file open and save
- Supports all agnix validation rules (99 rules)
- Maps diagnostic severity levels (Error, Warning, Info)
- Rule codes shown in diagnostic messages

## Supported File Types

The LSP server validates the same file types as the CLI:

- `SKILL.md` - Agent skill definitions
- `CLAUDE.md`, `AGENTS.md` - Claude Code memory files
- `.claude/settings.json` - Hook configurations
- `plugin.json` - Plugin manifests
- `*.mcp.json` - MCP tool configurations
- `.github/copilot-instructions.md` - GitHub Copilot instructions
- `.cursor/rules/*.mdc` - Cursor project rules

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
│   ├── backend.rs          # LSP protocol implementation
│   └── diagnostic_mapper.rs # Converts agnix diagnostics to LSP format
└── tests/
    └── lsp_integration.rs  # Integration tests
```

## License

MIT OR Apache-2.0
