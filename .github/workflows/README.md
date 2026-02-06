# GitHub Actions Workflows

This directory contains CI/CD workflows for the agnix project.

## Security Hardening

All workflows follow security best practices:

### 1. Explicit Permissions

Every workflow declares minimum required permissions at the workflow level.
Jobs that need additional permissions declare them at the job level.

- `permissions: {}` - No permissions (used when jobs specify their own)
- `permissions: contents: read` - Read-only access to repository contents

### 2. SHA-Pinned Actions

All third-party actions are pinned to specific commit SHAs to prevent
supply chain attacks. The SHA pins are documented with version comments
for maintainability.

### 3. Cache Save Restrictions

Rust caches (`Swatinem/rust-cache`) are configured with `save-if` conditions
to only save caches on protected branches (main) or tag pushes. This prevents
cache poisoning from pull requests.

## SHA Pin Reference

When updating actions, use these SHA commits (last verified: 2025-02):

```yaml
# GitHub Official Actions
actions/checkout@v4:           34e114876b0b11c390a56381ad16ebd13914f8d5
actions/upload-artifact@v4:    ea165f8d65b6e75b540449e92b4886f43607fa02
actions/download-artifact@v4:  d3f86a106a0bac45b974a628896c90dbdf5c8093
actions/setup-python@v5:       a26af69be951a213d495a4c3e4e4022e16d87065
actions/setup-node@v4:         49933ea5288caeca8642d1e84afbd3f7d6820020
actions/configure-pages@v5:    983d7736d9b0ae728b81ab479565c72886d7745b
actions/upload-pages-artifact@v3: 56afc609e74202658d3ffba0e8f6dda462b719fa
actions/deploy-pages@v4:       d6db90164ac5ed86f2b6aed7e0febac5b3c0c03e
rhysd/actionlint@v1.7.1:       62dc61a45fc95efe8c800af7a557ab0b9165d63b

# Rust Tooling
dtolnay/rust-toolchain@stable: 4be9e76fd7c4901c61fb841f559994984270fce7
Swatinem/rust-cache@v2:        779680da715d629ac1d338a641029a2f4372abb5
taiki-e/install-action@v2:     650c5ca14212efbbf3e580844b04bdccf68dac31
taiki-e/install-action@nextest: cd05dcd6eb73067dda063b97a15b7060049dacd9

# Security
github/codeql-action@v3:       2588666de8825e1e9dc4e2329a4c985457d55b32

# Coverage
codecov/codecov-action@v5:     671740ac38dd9b0130fbe1cec585b89eea48d3de

# Release
softprops/action-gh-release@v2: a06a81a03ee405af7f2048a818ed3f03bbf83c7b

# Claude Code
anthropics/claude-code-action@v1: 6867bb3ab0b2c0a10629b6823e457347e74ad6d2
```

## Updating Action Versions

When a new version of an action is released:

1. Check the release notes for security implications
2. Get the full SHA of the release tag:
   ```bash
   git ls-remote --tags https://github.com/owner/repo refs/tags/vX.Y.Z
   ```
3. Update all occurrences in workflow files
4. Update this README with the new SHA
5. Test the workflows on a feature branch before merging

## Workflow Overview

| Workflow | Trigger | Purpose |
|----------|---------|---------|
| ci.yml | push/PR to main | Lint, test, coverage, build |
| release.yml | tag push (v*) | Build and publish releases |
| security.yml | push/PR/schedule | CodeQL analysis and security audit |
| test-action.yml | push/PR (action paths) | Test the GitHub Action |
| changelog.yml | PR | Verify CHANGELOG.md is updated |
| claude.yml | issue/PR comments | Claude Code assistant |
| claude-code-review.yml | PR | Automated code review |
| spec-drift.yml | schedule/manual | Monitor upstream specs for changes |
| mcp-release-watch.yml | daily/manual | Watch MCP spec repo for new releases |
| docs-site.yml | push/PR/manual | Build and deploy documentation website |
