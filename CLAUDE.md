# Project Memory: agnix

> Linter for agent configurations. Validates Skills, Hooks, MCP, Memory, Plugins.

**Repository**: https://github.com/avifenesh/agnix

## Critical Rules

1. **Rust workspace** - agnix-core (lib) + agnix-cli (binary)
2. **Knowledge base is source of truth** - Rules in `knowledge-base/VALIDATION-RULES.md`
3. **Plain text output** - No emojis, no ASCII art
4. **Certainty filtering** - HIGH (>95%), MEDIUM (75-95%), LOW (<75%)
5. **Single binary** - Compile with LTO, strip symbols,
6. **Track work in GitHub issues** - All tasks tracked there
7. **Task is not done until tests added** - Every feature/fix must have quality tests
8. **Documentation** - Update README, SPEC.md, RULES.md as needed, dont use CLAUDE.md for docs and tracking
9. **Always follow the skill/command flow as instructed** - No deviations

## Architecture

```
crates/
├── agnix-core/     # Parsers, schemas, rules, diagnostics
└── agnix-cli/      # CLI with clap
knowledge-base/     # 74 rules, 75+ sources
tests/fixtures/     # Test cases
```

## Commands

```bash
cargo check                 # Compile check
cargo test                  # Run tests
cargo build --release       # Build binary
cargo run --bin agnix -- .  # Run CLI
```

## Rules Reference

74 rules in `knowledge-base/VALIDATION-RULES.md`

Format: `[CATEGORY]-[NUMBER]` (AS-004, CC-HK-001, etc.)

## Current State

- Compiles and runs with full validation pipeline
- `validate_project()` walks directories, detects file types, dispatches validators
- 45 passing tests
- See GitHub issues for remaining tasks

## References

- SPEC.md - Technical reference
- knowledge-base/INDEX.md - Knowledge navigation
- https://agentskills.io
- https://modelcontextprotocol.io
