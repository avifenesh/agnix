# Project Memory: agnix

> Linter for agent configurations. Validates Skills, Hooks, MCP, Memory, Plugins.

**Repository**: https://github.com/avifenesh/agnix

## Project Instruction Files

- `CLAUDE.md` is the project memory entrypoint for Claude Code.
- `AGENTS.md` is a byte-for-byte copy of `CLAUDE.md` for tools that read `AGENTS.md` (Codex CLI, OpenCode, Cursor, Cline, Copilot).
- Keep them identical (tests enforce this).

## Critical Rules

1. **Rust workspace** - agnix-core (lib) + agnix-cli (binary)
2. **rules.json is source of truth** - `knowledge-base/rules.json` is the machine-readable source of truth. When adding a new rule, add it to BOTH `rules.json` AND `VALIDATION-RULES.md`. CI parity tests enforce this.
3. **Plain text output** - No emojis, no ASCII art
4. **Certainty filtering** - HIGH (>95%), MEDIUM (75-95%), LOW (<75%)
5. **Single binary** - Compile with LTO, strip symbols,
6. **Track work in GitHub issues** - All tasks tracked there
7. **Task is not done until tests added** - Every feature/fix must have quality tests
8. **Documentation** - Keep long-form docs in `README.md`, `SPEC.md`, and `knowledge-base/` (especially `knowledge-base/VALIDATION-RULES.md`). Keep `CLAUDE.md`/`AGENTS.md` for agent instructions only.
9. **Always follow the skill/command flow as instructed** - No deviations
10. **No unnecessary files** - Don't create summary files, plan files, or temp docs unless specifically required
11. **Never merge without waiting for claude workflow to end successfully** - It might take time, but this is the major quality gate, and most thorough review.
12. **You MUST follow the flow phases one by one** - If they state to use subagents, tools, or any specific method, you must follow it exactly as described.

## Architecture

```
crates/
├── agnix-core/     # Parsers, schemas, rules, diagnostics
└── agnix-cli/      # CLI with clap
knowledge-base/     # 99 rules, 75+ sources
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

99 rules defined in `knowledge-base/rules.json` (source of truth)

Human-readable docs: `knowledge-base/VALIDATION-RULES.md`

Format: `[CATEGORY]-[NUMBER]` (AS-004, CC-HK-001, etc.)

**Adding a new rule**: Add to BOTH `rules.json` AND `VALIDATION-RULES.md`. CI parity tests will fail if they drift. Each rule in `rules.json` must include complete `evidence` metadata (source_type, source_urls, verified_on, applies_to, normative_level, tests). See VALIDATION-RULES.md for the evidence schema reference.

## Current State

- Compiles and runs with full validation pipeline
- `validate_project()` walks directories, detects file types, dispatches validators
- 663 passing tests
- See GitHub issues for remaining tasks

## Top tier tools support:

### S (test always)

- Claude code
- codex cli
- opencode

### A (test on major changes)

- GitHub copilot
- Cline
- Cursor

### B (test on significant changes if time permits)

- Roo Code
- Kiro cli
- amp
- pi

### C (Community reports fixes only)

- gemini cli
- continue
- Antigravity

## D (No support, nice to have, can try once in a while, mainly if users request)

- Tabnine
- Codeium
- Amazon Q
- Windsurf
- Aider
- SourceGraph Cody

## E (No support, do not test, full community support and contributions)

- Everything else

## References

- SPEC.md - Technical reference
- knowledge-base/INDEX.md - Knowledge navigation
- https://agentskills.io
- https://modelcontextprotocol.io
