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

### Crate Dependency Graph

```
agnix-rules (data-only, generated from rules.json)
    ↓
agnix-core (validation engine)
    ↓
├── agnix-cli (command-line interface)
└── agnix-lsp (language server protocol)
```

### Project Layout

```
crates/
├── agnix-rules/    # Rule definitions (build-time generated)
├── agnix-core/     # Core: parsers, schemas, validators, diagnostics
├── agnix-cli/      # CLI binary (clap)
└── agnix-lsp/      # LSP server (tower-lsp, tokio)
editors/
└── vscode/         # VS Code extension
knowledge-base/     # 100 rules, 75+ sources, rules.json
tests/fixtures/     # Test cases by category
```

### Core Modules (agnix-core)

- `parsers/` - Frontmatter, JSON, Markdown parsing
- `schemas/` - Type definitions (12 schemas: skill, hooks, agent, mcp, etc.)
- `rules/` - Validators implementing Validator trait (13 validators)
- `config.rs` - LintConfig, ToolVersions, SpecRevisions, ImportCache
- `diagnostics.rs` - Diagnostic, Fix, DiagnosticLevel
- `fixes.rs` - Auto-fix application engine
- `fs.rs` - FileSystem trait abstraction (RealFileSystem, MockFileSystem)

### Key Abstractions

```rust
// Primary extension point - implement for custom validators
pub trait Validator {
    fn validate(&self, path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic>;
}

// Registry pattern - multiple validators per FileType
pub struct ValidatorRegistry {
    validators: HashMap<FileType, Vec<ValidatorFactory>>
}

// Diagnostic with optional auto-fix
pub struct Diagnostic {
    pub level: DiagnosticLevel,
    pub rule: String,        // "AS-004", "CC-SK-001"
    pub fixes: Vec<Fix>,     // Byte-range replacements
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

```rust
// Backend caches config and registry for performance
pub struct Backend {
    config: RwLock<Arc<LintConfig>>,
    registry: Arc<ValidatorRegistry>,  // Immutable, shared
    documents: RwLock<HashMap<Url, String>>,  // Content cache
}
```

- Validation runs in `spawn_blocking()` (CPU-bound, sync)
- Events: `did_open`, `did_change`, `did_save`, `codeAction`, `hover`

## Commands

```bash
cargo check                 # Compile check
cargo test                  # Run tests
cargo build --release       # Build binary
cargo run --bin agnix -- .  # Run CLI
```

## Rules Reference

100 rules defined in `knowledge-base/rules.json` (source of truth)

Human-readable docs: `knowledge-base/VALIDATION-RULES.md`

Format: `[CATEGORY]-[NUMBER]` (AS-004, CC-HK-001, etc.)

**Adding a new rule**: Add to BOTH `rules.json` AND `VALIDATION-RULES.md`. CI parity tests will fail if they drift. Each rule in `rules.json` must include complete `evidence` metadata (source_type, source_urls, verified_on, applies_to, normative_level, tests). See VALIDATION-RULES.md for the evidence schema reference.

## Current State

- v0.2.0 - Production-ready with full validation pipeline
- 100 validation rules across 13 validators
- 1250+ passing tests
- LSP server with VS Code extension
- See GitHub issues for roadmap

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
