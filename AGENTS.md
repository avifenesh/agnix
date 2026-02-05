# Project Memory: agnix

> Linter for agent configurations. Validates Skills, Hooks, MCP, Memory, Plugins.

**Repository**: https://github.com/avifenesh/agnix

## Project Instruction Files

- `CLAUDE.md` is the project memory entrypoint for Claude Code.
- `AGENTS.md` is a byte-for-byte copy of `CLAUDE.md` for tools that read `AGENTS.md` (Codex CLI, OpenCode, Cursor, Cline, Copilot).
- Keep them identical (tests enforce this).

## Critical Rules

1. **Rust workspace** - agnix-rules (data), agnix-core (lib), agnix-cli/agnix-lsp/agnix-mcp (binaries)
2. **rules.json is source of truth** - `knowledge-base/rules.json` is the machine-readable source of truth. When adding a new rule, add it to BOTH `rules.json` AND `VALIDATION-RULES.md`. CI parity tests enforce this.
3. **Plain text output** - No emojis, no ASCII art
4. **Certainty filtering** - HIGH (>95%), MEDIUM (75-95%), LOW (<75%)
5. **Release binaries** - Compile with LTO, strip symbols
6. **Track work in GitHub issues** - All tasks tracked there
7. **Task is not done until tests added** - Every feature/fix must have quality tests
8. **Documentation** - Keep long-form docs in `README.md`, `SPEC.md`, and `knowledge-base/` (especially `knowledge-base/VALIDATION-RULES.md`). Keep `CLAUDE.md`/`AGENTS.md` for agent instructions only.
9. **Always follow the skill/command flow as instructed** - No deviations
10. **No unnecessary files** - Don't create summary files, plan files, or temp docs unless specifically required
11. **Never merge without waiting for claude workflow to end successfully** - It might take time, but this is the major quality gate, and most thorough review.
12. **You MUST follow the flow phases one by one** - If they state to use subagents, tools, or any specific method, you must follow it exactly as described.
13. **You MUST address all comments and reviews** - If reviewers leave comments, even minor ones, and even if not a requested change, you must address them all before merging. If you disagree, respond in the review comments. Minor comments must still be addressed.

## Architecture

### Crate Dependency Graph

```
agnix-rules (data-only, generated from rules.json)
    ↓
agnix-core (validation engine)
    ↓
├── agnix-cli (command-line interface)
├── agnix-lsp (language server protocol)
└── agnix-mcp (MCP server)
```

### Project Layout

```
crates/
├── agnix-rules/    # Rule definitions (build-time generated)
├── agnix-core/     # Core: parsers, schemas, validators, diagnostics
├── agnix-cli/      # CLI binary (clap)
├── agnix-lsp/      # LSP server (tower-lsp, tokio)
└── agnix-mcp/      # MCP server (rmcp)
editors/
└── vscode/         # VS Code extension
knowledge-base/     # 100 rules, 75+ sources, rules.json
tests/fixtures/     # Test cases by category
```

### Core Modules (agnix-core)

- `parsers/` - Frontmatter, JSON, Markdown parsing
- `schemas/` - Type definitions (11 schemas: skill, hooks, agent, mcp, etc.)
- `rules/` - Validators implementing Validator trait (13 validators)
- `config.rs` - LintConfig, ToolVersions, SpecRevisions
- `diagnostics.rs` - Diagnostic, Fix, DiagnosticLevel
- `eval.rs` - Rule efficacy evaluation (precision/recall/F1)
- `file_utils.rs` - Safe file I/O (symlink rejection, size limits)
- `fixes.rs` - Auto-fix application engine
- `fs.rs` - FileSystem trait abstraction (RealFileSystem, MockFileSystem)

### Key Abstractions

```rust
// Primary extension point
pub trait Validator {
    fn validate(&self, path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic>;
}

// Multiple validators per FileType via factory pattern
pub struct ValidatorRegistry {
    validators: HashMap<FileType, Vec<ValidatorFactory>>
}
```

### Validation Flow

```
CLI args → LintConfig → validate_project()
    → Directory walk (ignore crate, respects .gitignore)
    → detect_file_type() per file (path-based, no I/O)
    → Parallel validation (rayon)
    → Validators from registry run sequentially per file
    → Post-processing (AGM-006, XP-004/005/006)
    → Output (text/JSON/SARIF)
```

### LSP Architecture

- Backend holds `RwLock<Arc<LintConfig>>`, immutable `Arc<ValidatorRegistry>`, document cache
- Validation runs in `spawn_blocking()` (CPU-bound, sync)
- Events: `did_open`, `did_change`, `did_save`, `did_close`, `did_change_configuration`, `codeAction`, `hover`

## Commands

```bash
cargo check                 # Compile check
cargo test                  # Run tests
cargo build --release       # Build binaries
cargo run --bin agnix -- .  # Run CLI
cargo run --bin agnix-lsp   # Run LSP server
cargo run --bin agnix-mcp   # Run MCP server
```

## Rules Reference

100 rules defined in `knowledge-base/rules.json` (source of truth)

Human-readable docs: `knowledge-base/VALIDATION-RULES.md`

Format: `[CATEGORY]-[NUMBER]` (AS-004, CC-HK-001, etc.)

**Adding a new rule**: Add to BOTH `rules.json` AND `VALIDATION-RULES.md`. CI parity tests will fail if they drift. Each rule in `rules.json` must include complete `evidence` metadata (source_type, source_urls, verified_on, applies_to, normative_level, tests). See VALIDATION-RULES.md for the evidence schema reference.

## Current State

- v0.7.2 - Production-ready with full validation pipeline
- 100 validation rules across 13 validators
- 1600+ passing tests
- LSP + MCP servers with VS Code extension
- See GitHub issues for roadmap

## Tool Support Tiers

- **S** (test always): Claude Code, Codex CLI, OpenCode
- **A** (test on major changes): GitHub Copilot, Cline, Cursor
- **B** (test if time permits): Roo Code, Kiro CLI, amp, pi
- **C** (community reports only): gemini cli, continue, Antigravity
- **D** (nice to have): Tabnine, Codeium, Amazon Q, Windsurf, Aider, SourceGraph Cody
- **E** (no support, community contributions): Everything else

## References

- SPEC.md - Technical reference
- knowledge-base/INDEX.md - Knowledge navigation
- https://agentskills.io
- https://modelcontextprotocol.io
