---
title: Editor Integration
description: "Set up agnix real-time diagnostics in VS Code, Neovim, JetBrains, and Zed."
---

# Editor Integration

agnix ships an LSP server (`agnix-lsp`) that provides real-time diagnostics, code actions, and hover documentation in your editor.

## Capabilities

- Diagnostics on open, save, and change
- Code actions for fixable findings
- Hover details for rule explanations

## VS Code

Install the extension from the
[VS Code Marketplace](https://marketplace.visualstudio.com/items?itemName=avifenesh.agnix).

The extension bundles the LSP server. No additional setup needed.

For manual configuration, see the
[VS Code extension README](https://github.com/avifenesh/agnix/tree/main/editors/vscode).

## JetBrains (IntelliJ, WebStorm, etc.)

Install from the
[JetBrains Plugin Marketplace](https://plugins.jetbrains.com/plugin/30087-agnix).

Configure the `agnix-lsp` binary path in plugin settings if not auto-detected.

For details, see the
[JetBrains plugin README](https://github.com/avifenesh/agnix/tree/main/editors/jetbrains).

## Neovim

Use the Neovim plugin with your LSP client (nvim-lspconfig, mason, etc.):

```lua
-- With nvim-lspconfig
require('lspconfig').agnix.setup{}
```

Install the LSP binary:

```bash
cargo install agnix-lsp
```

For full setup instructions, see the
[Neovim plugin README](https://github.com/avifenesh/agnix/tree/main/editors/neovim).

## Zed

Install the agnix extension from the Zed extension marketplace, or see the
[Zed extension README](https://github.com/avifenesh/agnix/tree/main/editors/zed).

## Other editors

Any editor with LSP support can use `agnix-lsp`. Point your LSP client to the binary:

```bash
cargo install agnix-lsp
agnix-lsp
```

For the full editor support matrix, see
[docs/EDITOR-SETUP.md](https://github.com/avifenesh/agnix/blob/main/docs/EDITOR-SETUP.md).
