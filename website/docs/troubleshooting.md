---
title: Troubleshooting
description: "Common issues and fixes when using agnix CLI and editor integrations."
---

# Troubleshooting

## agnix command not found

Ensure `agnix` is in your PATH:

```bash
which agnix
agnix --version
```

If installed via npm, make sure npm global binaries are in your PATH. Run `npm config get prefix` to find the install location.

## No files found to validate

agnix respects `.gitignore` and has file discovery boundaries. Check:

- You are running from the project root
- Your config files are not git-ignored
- `.agnix.toml` `target` or `tools` settings are not excluding your files

## Unexpected rules triggering

Check your `.agnix.toml` configuration:

- `target` limits validation to a single tool's files
- `disabled_rules` can suppress specific rules
- Some rules only apply to specific tool configs

## LSP diagnostics not showing

1. Verify the `agnix-lsp` binary is installed and accessible:
   ```bash
   which agnix-lsp
   agnix-lsp --version
   ```

2. Check your editor plugin points to the correct binary path

3. Check editor logs for LSP server startup errors

4. For VS Code, the extension bundles the server - try reinstalling the extension

## Rule behavior differs from docs

Validate against the canonical rule data:

- [knowledge-base/rules.json](https://github.com/avifenesh/agnix/blob/main/knowledge-base/rules.json)
- [knowledge-base/VALIDATION-RULES.md](https://github.com/avifenesh/agnix/blob/main/knowledge-base/VALIDATION-RULES.md)

If you find a discrepancy, please [open an issue](https://github.com/avifenesh/agnix/issues/new).

## Auto-fix changed something unexpected

Run `agnix --fix` on a clean git working tree so you can review changes with `git diff`. If a fix is incorrect, [report it](https://github.com/avifenesh/agnix/issues/new) with the original file content.
