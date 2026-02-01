# Implementation Plan: Task #48 - AGENTS.md Rules (AGM-001..AGM-006)

## Overview

Implement six AGM validation rules for AGENTS.md files focusing on cross-platform compatibility. AGENTS.md is used by multiple AI coding tools (Codex CLI, OpenCode, GitHub Copilot, Cursor, Cline).

## Rules to Implement

| Rule | Certainty | Description |
|------|-----------|-------------|
| AGM-001 | HIGH | Valid Markdown Structure - check for unclosed code blocks, malformed links |
| AGM-002 | MEDIUM | Missing Section Headers - verify presence of # or ## headers |
| AGM-003 | HIGH | Character Limit - content should be under 12000 chars (Windsurf) |
| AGM-004 | MEDIUM | Missing Project Context - should describe project purpose/stack |
| AGM-005 | HIGH | Platform-Specific Features Without Guard - features need guard comments |
| AGM-006 | MEDIUM | Nested AGENTS.md Hierarchy - multiple AGENTS.md in directory tree |

## Implementation Steps

### Step 1: Create Detection Helpers
**File:** `crates/agnix-core/src/schemas/agents_md.rs`

Create detection functions using OnceLock<Regex> pattern:
- `check_markdown_validity()` - detect unclosed code blocks, malformed syntax
- `check_section_headers()` - verify headers presence
- `check_character_limit()` - return char count vs 12000 limit
- `check_project_context()` - look for project description patterns
- `find_unguarded_platform_features()` - detect unguarded platform content
- `find_nested_agents_md()` - analyze path list for hierarchy

### Step 2: Create Validator
**File:** `crates/agnix-core/src/rules/agents_md.rs`

Implement `AgentsMdValidator` following mcp.rs pattern:
- Check filename == 'AGENTS.md' (skip CLAUDE.md)
- Implement rules AGM-001 through AGM-005 (file-level)
- HIGH certainty -> Diagnostic::error()
- MEDIUM certainty -> Diagnostic::warning()
- Unit tests for each rule

### Step 3: Wire Up Modules
**Files:**
- `crates/agnix-core/src/rules/mod.rs` - add `pub mod agents_md;`
- `crates/agnix-core/src/schemas/mod.rs` - add `pub mod agents_md;`

### Step 4: Add Config Support
**File:** `crates/agnix-core/src/config.rs`

- Add `pub agents_md: bool` to `RuleConfig`
- Add match arm for `AGM-` prefix in `is_category_enabled()`
- Add tests for config enablement

### Step 5: Register Validator
**File:** `crates/agnix-core/src/lib.rs`

- Import `AgentsMdValidator`
- Add to `FileType::ClaudeMd` validators list
- Update test expectations

### Step 6: Implement AGM-006 Project-Level Detection
**Files:**
- `crates/agnix-core/src/schemas/agents_md.rs` - `find_nested_agents_md()` helper
- `crates/agnix-core/src/lib.rs` - call from `validate_project()`

### Step 7: Create Test Fixtures
**Directory:** `tests/fixtures/agents_md/`

- `valid/AGENTS.md` - all best practices
- `no-context/AGENTS.md` - missing project section (AGM-004)
- `too-large/AGENTS.md` - over 12000 chars (AGM-003)
- `invalid-markdown/AGENTS.md` - malformed syntax (AGM-001)
- `no-headers/AGENTS.md` - plain text (AGM-002)
- `unguarded-platform/AGENTS.md` - unguarded features (AGM-005)
- `nested/AGENTS.md` + `nested/subdir/AGENTS.md` - hierarchy (AGM-006)

### Step 8: Add Integration Tests
**File:** `crates/agnix-core/src/lib.rs`

- Test each AGM rule fires correctly
- Test AGM rules don't fire for CLAUDE.md
- Test config disablement
- Test AGM-006 project-level detection

### Step 9: Verify All Tests Pass
- `cargo test`
- `cargo clippy`
- `cargo build --release`

## Key Decisions

1. **AGM vs XP distinction**: AGM rules validate structure/content quality; XP rules detect platform-specific features
2. **AGM-006 at project level**: Requires directory tree analysis, implemented in `validate_project()`
3. **Filename filtering**: Validator checks filename to skip CLAUDE.md files

## Risks

- AGM-001 scope: Define clear criteria for "valid markdown" (unclosed blocks, broken links)
- XP-002 overlap: AGM-002 focuses on headers presence, XP-002 on hierarchy
- Performance: AGM-006 path hierarchy check needs efficient implementation

## Complexity: Medium | Confidence: High
