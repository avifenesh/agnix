# Releasing agnix

## Version Bumping

1. Update version in all `Cargo.toml` files:
   ```bash
   # Workspace root
   grep -rn 'version = "0\.' Cargo.toml crates/*/Cargo.toml
   # Update each to the new version
   ```

2. Update `CHANGELOG.md` with the new version section.

3. Commit version bump:
   ```bash
   git add -A
   git commit -m "release: v0.X.Y"
   ```

## Build Release Binaries

Release builds use LTO and stripped symbols (per project rules):

```bash
cargo build --release
```

The binaries are at:
- `target/release/agnix` (CLI)
- `target/release/agnix-lsp` (LSP server)
- `target/release/agnix-mcp` (MCP server)

## Pre-release Checks

```bash
# All tests pass
cargo test --workspace

# Doc tests
cargo test --doc --workspace

# Clippy clean
cargo clippy --workspace -- -D warnings

# Eval passes (41/42 minimum, 1 pre-existing XP-001 failure)
cargo run --bin agnix -- eval tests/eval.yaml

# Self-lint (agnix validates its own config)
cargo run --bin agnix -- .
```

## Creating a GitHub Release

1. Tag the release:
   ```bash
   git tag -a v0.X.Y -m "Release v0.X.Y"
   git push origin v0.X.Y
   ```

2. The GitHub Actions release workflow will automatically:
   - Build binaries for Linux (x86_64, aarch64), macOS (x86_64, aarch64), Windows (x86_64)
   - Create a GitHub Release with the binaries
   - Publish the VS Code extension (if applicable)

3. Verify the release at https://github.com/avifenesh/agnix/releases

## Post-release Verification

After the release workflow completes, verify all install targets work. This should be automated via a post-release CI workflow.

### Install Targets to Verify

| Target | Install Command | Verify Command |
|--------|----------------|----------------|
| **Cargo** | `cargo install agnix` | `agnix --version` |
| **Homebrew** | `brew install agnix` | `agnix --version` |
| **npm** | `npm install -g @agnix/cli` | `agnix --version` |
| **GitHub Release** | Download from releases page | Run binary directly |

### Editor Extensions to Verify

| Editor | Install Method | Verify |
|--------|---------------|--------|
| **VS Code** | Marketplace or `code --install-extension` | Open a CLAUDE.md, check diagnostics appear |
| **JetBrains** | Plugin marketplace | Open a CLAUDE.md, check diagnostics appear |
| **Neovim** | Plugin manager (lazy.nvim, etc.) | `:LspInfo` shows agnix-lsp attached |
| **Zed** | Extension marketplace | Open a CLAUDE.md, check diagnostics appear |

### Post-release CI (Ideal)

A `post-release.yml` workflow triggered on release publication should:
1. Install from each distribution channel (cargo, brew, npm)
2. Run `agnix --version` to verify correct version
3. Run `agnix` against a small test fixture to verify basic functionality
4. Verify editor extension marketplace listings are updated
5. Verify documentation website is deployed with new version

### Manual Checklist

- [ ] GitHub Release page shows all platform binaries
- [ ] `cargo install agnix` installs the new version
- [ ] VS Code extension downloads the new LSP binary
- [ ] Documentation website shows the new version
- [ ] CHANGELOG.md is up to date
- [ ] Announce on relevant channels
- [ ] Close any milestone issues tied to this release

## Documentation & Website

Before release:
1. Ensure `website/versions.json` includes the new version
2. Copy the latest versioned docs: `cp -r website/versioned_docs/version-X.Y.Z website/versioned_docs/version-NEW`
3. Copy sidebars: `cp website/versioned_sidebars/version-X.Y.Z-sidebars.json website/versioned_sidebars/version-NEW-sidebars.json`
4. Regenerate rule docs: `python3 scripts/generate-docs-rules.py`
5. Verify docs build: `cd website && npm run build`

After release:
- The docs-site workflow deploys automatically on merge to main
- Verify at https://agentskills.io that new version docs are live
- Check that rule reference pages match the current rules.json

## Versioning Policy

- **Patch** (0.X.Y): Bug fixes, false positive/negative improvements, message quality
- **Minor** (0.X.0): New rules, new file type support, new validators
- **Major** (X.0.0): Breaking changes to config format, CLI interface, or rule IDs
