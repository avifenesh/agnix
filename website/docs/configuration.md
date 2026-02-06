# Configuration

Use `.agnix.toml` in repository root.

```toml
target = "claude-code"
strict = false
max_files = 10000
locale = "en"
disabled_rules = []
```

Common options:

- `target`: single tool focus (`claude-code`, `cursor`, `codex`, etc.)
- `tools`: multi-tool targeting
- `strict`: treat warnings as errors
- `fix`: apply available fixes
- `locale`: output locale

Authoritative reference:

- [docs/CONFIGURATION.md](https://github.com/avifenesh/agnix/blob/main/docs/CONFIGURATION.md)
