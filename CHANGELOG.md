# Changelog

All notable changes to agnix will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- SARIF 2.1.0 output format with `--format sarif` CLI option for CI/CD integration
  - Full SARIF 2.1.0 specification compliance with JSON schema validation
  - Includes all 80 validation rules in driver.rules with help URIs
  - Supports GitHub Code Scanning and other SARIF-compatible tools
  - Proper exit codes for CI workflows (errors exit 1)
  - Path normalization for cross-platform compatibility
  - 8 comprehensive integration tests for SARIF output
- SkillValidator Claude Code rules (CC-SK-001 to CC-SK-005, CC-SK-008 to CC-SK-009)
  - CC-SK-001: Validates model field values (sonnet, opus, haiku, inherit)
  - CC-SK-002: Validates context field must be 'fork' or omitted
  - CC-SK-003: Requires 'agent' field when context is 'fork'
  - CC-SK-004: Requires 'context: fork' when agent field is present
  - CC-SK-005: Validates agent type values (Explore, Plan, general-purpose)
  - CC-SK-006: Dangerous skills must set 'disable-model-invocation: true'
  - CC-SK-007: Warns on unrestricted Bash access (suggests scoped versions)
  - CC-SK-008: Validates tool names in allowed-tools against known Claude Code tools
  - CC-SK-009: Warns when too many dynamic injections (!`) detected (>3)
- 27 comprehensive unit tests for skill validation (244 total tests)
- 9 test fixtures in tests/fixtures/skills/ directory for CC-SK rules
- JSON output format with `--format json` CLI option for programmatic consumption
  - Simple, human-readable structure for easy parsing and integration
  - Includes version, files_checked, diagnostics array, and summary counts
  - Cross-platform path normalization (forward slashes)
  - Proper exit codes for CI workflows (errors exit 1)
  - 14 comprehensive unit tests for JSON output
- Comprehensive CI workflow with format check, clippy, machete, and test matrix (3 OS x 2 Rust versions)
- Security scanning workflow with CodeQL analysis and cargo-audit (runs on push, PR, and weekly schedule)
- Changelog validation workflow to ensure CHANGELOG.md is updated in PRs
- PluginValidator implementation with 5 validation rules (CC-PL-001 to CC-PL-005)
  - CC-PL-001: Validates plugin.json is in .claude-plugin/ directory
  - CC-PL-002: Detects misplaced components (skills/agents/hooks) inside .claude-plugin/
  - CC-PL-003: Validates version uses semver format (X.Y.Z)
  - CC-PL-004: Validates required fields (name, description, version)
  - CC-PL-005: Validates name field is not empty
- Path traversal protection with MAX_TRAVERSAL_DEPTH limit
- 47 comprehensive tests for plugin validation (234 total tests)
- 4 test fixtures in tests/fixtures/plugins/ directory
- Auto-fix infrastructure with CLI flags:
  - `--fix`: Apply automatic fixes to detected issues
  - `--dry-run`: Preview fixes without modifying files
  - `--fix-safe`: Only apply high-certainty (safe) fixes
- `Fix` struct with `FixKind` enum (Replace, Insert, Delete) in diagnostics
- `apply_fixes()` function to process and apply fixes to files
- Diagnostics now include `[fixable]` marker in output for issues with available fixes
- Hint message in CLI output when fixable issues are detected
- Config-based rule filtering with category toggles (skills, hooks, agents, memory, plugins, xml, imports)
- Target tool filtering - CC-* rules automatically disabled for non-Claude Code targets (Cursor, Codex)
- Individual rule disabling via `disabled_rules` config list
- `is_rule_enabled()` method with category and target awareness
- AgentValidator implementation with 6 validation rules (CC-AG-001 to CC-AG-006)
  - CC-AG-001: Validates required 'name' field in agent frontmatter
  - CC-AG-002: Validates required 'description' field in agent frontmatter
  - CC-AG-003: Validates model values (sonnet, opus, haiku, inherit)
  - CC-AG-004: Validates permissionMode values (default, acceptEdits, dontAsk, bypassPermissions, plan)
  - CC-AG-005: Validates referenced skills exist at .claude/skills/[name]/SKILL.md
  - CC-AG-006: Detects conflicts between 'tools' and 'disallowedTools' arrays
- Path traversal security protection for skill name validation
- 44 comprehensive tests for agent validation (152 total tests)
- 7 test fixtures in tests/fixtures/agents/ directory
- Parallel file validation using rayon for improved performance on large projects
- Deterministic diagnostic output with sorting by severity and file path
- Comprehensive tests for parallel validation edge cases

### Changed
- `validate_project()` now processes files in parallel while maintaining deterministic output
- Directory walking remains sequential, only validation is parallelized
- All validators now respect config-based category toggles and disabled rules
- Config structure enhanced with category-based toggles (legacy flags still supported)
- Knowledge base docs refreshed (rule counts, AGENTS.md support tiers, Cursor rules)

### Performance
- Significant speed improvements on projects with many files
- Maintains correctness with deterministic sorting of results
