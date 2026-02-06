---
title: Installation
description: "Install agnix via npm, Homebrew, Cargo, or download pre-built binaries."
---

# Installation

## npm (recommended)

Works on all platforms. Includes pre-built binaries.

```bash
npm install -g agnix
```

Or run without installing:

```bash
npx agnix .
```

## Homebrew (macOS / Linux)

```bash
brew tap avifenesh/agnix && brew install agnix
```

## Cargo (Rust toolchain)

```bash
cargo install agnix-cli
```

## Pre-built binaries

Download from [GitHub Releases](https://github.com/avifenesh/agnix/releases) for your platform.

## Verify installation

```bash
agnix --version
```

## Editor extensions

agnix ships editor integrations powered by the `agnix-lsp` server:

| Editor | Install |
|--------|---------|
| VS Code | [Marketplace](https://marketplace.visualstudio.com/items?itemName=avifenesh.agnix) |
| JetBrains | [Plugin](https://plugins.jetbrains.com/plugin/30087-agnix) |
| Neovim | [Plugin](https://github.com/avifenesh/agnix/tree/main/editors/neovim) |
| Zed | [Extension](https://github.com/avifenesh/agnix/tree/main/editors/zed) |

See [Editor Integration](./editor-integration.md) for setup details.
