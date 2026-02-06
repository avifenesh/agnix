# Troubleshooting

## Command not found

Ensure `agnix` is in your PATH:

```bash
which agnix
agnix --version
```

## Unexpected validation scope

Check:

- `.agnix.toml` target/tools settings
- `.gitignore` and file discovery boundaries

## LSP diagnostics are missing

Check that editor plugin points to `agnix-lsp` binary and server logs do not show startup errors.

## Rule mismatch questions

If docs and rule behavior appear out of sync, validate against canonical rule data:

- [knowledge-base/rules.json](https://github.com/avifenesh/agnix/blob/main/knowledge-base/rules.json)
- [knowledge-base/VALIDATION-RULES.md](https://github.com/avifenesh/agnix/blob/main/knowledge-base/VALIDATION-RULES.md)
