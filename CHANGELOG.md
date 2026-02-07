# Changelog

All notable changes to agnix will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- CC-HK-013: `async` field only valid on command hooks (error)
- CC-HK-014: `once` field only meaningful in skill/agent frontmatter (warning)
- CC-HK-015: `model` field only valid on prompt/agent hooks (warning)
- CC-HK-016: Unknown hook type validation, recognizes `agent` type (error)
- CC-HK-017: Prompt/agent hooks missing `$ARGUMENTS` reference (warning)
- CC-HK-018: Matcher on `UserPromptSubmit`/`Stop` events silently ignored (info)
- CC-AG-008: Validate `memory` scope is `user`, `project`, or `local`
- CC-AG-009: Validate tool names in agent `tools` list
- CC-AG-010: Validate tool names in agent `disallowedTools` list
- CC-AG-011: Validate `hooks` object schema in agent frontmatter
- CC-AG-012: Warn on `permissionMode: bypassPermissions` usage
- CC-AG-013: Validate skill name format in agent `skills` array
- MCP-009: Validate `command` is present for stdio MCP servers (HIGH)
- MCP-010: Validate `url` is present for http/sse MCP servers (HIGH)
- MCP-011: Validate MCP server `type` is one of stdio, http, sse (HIGH)
- MCP-012: Warn when SSE transport is used (deprecated in favor of Streamable HTTP) with auto-fix (MEDIUM)
- CC-SK-010: Validate hooks in skill frontmatter follow settings.json schema
- CC-SK-011: Detect unreachable skills (user-invocable=false + disable-model-invocation=true)
- CC-SK-012: Warn when argument-hint is set but body never references $ARGUMENTS
- CC-SK-013: Warn when context=fork is used with reference-only content (no imperative verbs)
- CC-SK-014: Validate disable-model-invocation is boolean, not string "true"
- CC-SK-015: Validate user-invocable is boolean, not string "true"/"false"
- CC-PL-007: Validate component paths are relative (no absolute paths or `..` traversal) with safe auto-fix (HIGH)
- CC-PL-008: Detect component paths pointing inside `.claude-plugin/` directory (HIGH)
- CC-PL-009: Validate `author.name` is non-empty when author object present (MEDIUM)
- CC-PL-010: Validate `homepage` is a valid http/https URL when present (MEDIUM)
- COP-005: Validate `excludeAgent` field contains valid agent identifiers
- COP-006: Warn when global Copilot instruction file exceeds ~4000 characters
- CUR-007: Warn when `alwaysApply: true` is set alongside `globs` (redundant) with safe auto-fix
- CUR-008: Detect `alwaysApply` as quoted string instead of boolean (HIGH)
- CUR-009: Warn when agent-requested rule has no description
- OC-001: Validate `share` field in `opencode.json` is `"manual"`, `"auto"`, or `"disabled"` (HIGH)
- OC-002: Validate instruction paths in `opencode.json` exist or are valid globs (HIGH)
- OC-003: Validate `opencode.json` is parseable JSON/JSONC with line/column error reporting (HIGH)
- Fix metadata (`autofix`, `fix_safety`) for all 100 rules in rules.json
- Fix metadata schema validation parity test
- Autofix count parity test (rules.json vs VALIDATION-RULES.md)
- Context-aware completions documented in all editor READMEs
- `--fix-safe` flag documented in README.md usage section

### Changed
- Auto-fix table in VALIDATION-RULES.md expanded from 7 to 25 rules with safety classification
- Auto-fixable count corrected from 22 to 25 rules (25%)
- Generated website rule pages now include Auto-Fix metadata
- Website rules index table includes Auto-Fix column
- `generate-docs-rules.py` renders fix metadata with strict validation
## [0.9.3] - 2026-02-06

### Fixed
- VS Code extension now probes PATH binaries with `--version` and prefers up-to-date downloaded binary over outdated system installations
- Version check handles pre-0.9.2 agnix-lsp binaries without `--version` support
- Reordered `findLspBinary()` to prefer the downloaded binary when its version marker matches, skipping the `--version` probe on subsequent restarts

## [0.9.2] - 2026-02-06

### Added
- `agnix-lsp --version`/`-V` flag for debugging

### Fixed
- VS Code and JetBrains plugins now auto-update LSP binary when plugin version changes
- Plugin writes `.agnix-lsp-version` marker file to detect version mismatches
- GitHub release URLs use versioned paths instead of `/latest/` for reliable downloads

## [0.9.1] - 2026-02-06

### Fixed
- CC-MEM-006: Detect positive alternatives after negatives ("NEVER X - always Y" no longer false positive)
- PE-004: Skip ambiguous terms inside parentheses (descriptive text no longer flagged)
- CC-AG-007: Humanize YAML parse errors ("expected a YAML list" instead of "expected a sequence")
- MCP-002: Suggest `parameters` -> `inputSchema` when field exists under wrong name
- VS Code marketplace image now bundled in extension package
- Exclude DEVELOPER.md and 11 other developer-focused files from validation

### Added
- JetBrains plugin auto-publish in release workflow
- Zed extension auto-publish via zed-extension-action
- All editor extension versions now auto-synced from Cargo.toml on release

## [0.9.0] - 2026-02-06

### Changed
- Validated against 1,200+ real-world repositories with 71 rules triggered
- Exclude non-agent markdown files (README.md, docs/, wiki/) from validation
- Restrict REF-002 broken link detection to agent config files only
- Skip HTML5 void elements and markdown-safe elements in XML balance checking
- Resolve @imports relative to project root when file-relative fails
- Apply prompt quality rules (CC-MEM-005/006, PE-*) to Cursor rule files
- Detect .cursorrules.md as Cursor rules variant
- Flag `|| true` and `2>/dev/null` as error suppression in hooks (CC-HK-009)
- Broaden persona detection in CC-MEM-005 ("You're a senior...")
- Add PCRE assertions to AS-014 regex escape detection
- Fix %% formatting in diagnostic messages across all locales
- Reduce false positive rate from ~30% to <3% across XML, REF, and XP rules
- Skip type parameters and path template placeholders in XML validation
- Filter email domains, Java annotations, and social handles from @import detection

### Added
- `docs/RELEASING.md` - Release process guide with install target verification
- `docs/REAL-WORLD-TESTING.md` - Real-world validation and manual inspection guide
- `scripts/real-world-validate.py` - Batch validation harness
- `tests/real-world/repos.yaml` - Curated manifest of 1,236 repos
- Regression test fixtures for HTML5 void elements, type parameters, and absolute paths

## [0.8.1] - 2026-02-06

### Added
- Authoring metadata and completion system (`authoring` module) with context-aware suggestions and hover docs for all config file types
- LSP completion provider with intelligent key/value/snippet suggestions
- Auto-fix support across validators: skills (AS-005, AS-006, CC-SK-001, CC-SK-003, CC-SK-005), agents (CC-AG-003, CC-AG-004), hooks (CC-HK-011), plugins (CC-PL-005), MCP (MCP-001)
- Safety tagging for all auto-fixes (safe vs unsafe)

### Changed
- LSP hover provider simplified by delegating to `agnix_core::authoring` module
- Agent and skill validators now use `split_frontmatter()` directly for better error location and fix generation

### Fixed
- CC-AG-007 parse error diagnostics now report the actual error line/column instead of always line 1

## [0.8.0] - 2026-02-06

### Added
- Real-world validation harness (`scripts/real-world-validate.py`) with 121 curated repos (`tests/real-world/repos.yaml`) (#184)
- XP-001: detect `@import` syntax in AGENTS.md files (Claude Code specific)
- XP-003: detect OS-specific absolute paths (`/Users/`, `/home/`, `~/Library/`, `~/.config/`)
- CC-MEM-005: detect role-play preambles and generic programming principles

### Changed
- Exclude non-agent markdown files from validation (README.md, CONTRIBUTING.md, docs/, wiki/, etc.) to reduce false positives by 57%
- Agent directory files (`agents/*.md`) take precedence over filename exclusions

### Fixed
- Operator precedence bug in `@import` email filtering that incorrectly matched email addresses
- Zed editor extension with automatic LSP binary download and MDC file type support (#198)
- Documentation website pipeline (#195)
  - Added Docusaurus website under `website/` with versioned docs and local search
  - Added rule-doc generation from `knowledge-base/rules.json` via `scripts/generate-docs-rules.py`
  - Added docs parity test (`crates/agnix-cli/tests/docs_website_parity.rs`) and CI workflow (`.github/workflows/docs-site.yml`)
- CI: code coverage reporting with cargo-llvm-cov and Codecov integration (#238)
- JetBrains plugin: archive extraction tests for AgnixBinaryDownloader (#255)
  - 19 tests covering TAR.GZ/ZIP extraction, binary selection, path traversal protection
  - Refactored extraction methods to companion object for testability
  - Switched path verification to `java.nio.file.Path` API for robustness
- Internationalization (i18n) support with rust-i18n (#207)
  - Support for multiple languages: English (en), Spanish (es), Chinese Simplified (zh-CN)
  - CLI flag `--locale` to set output language
  - CLI flag `--list-locales` to display available locales
  - Environment variable `AGNIX_LOCALE` for system-wide locale setting
  - Config field `locale` in `.agnix.toml` for project-specific locale
  - Automatic locale detection from system settings (LANG/LC_ALL)
  - LSP server locale initialization for editor integration
  - JSON and SARIF output always in English for CI/CD consistency
  - Translation guide in docs/TRANSLATING.md for contributors
  - Comprehensive test suite for locale detection and fallback behavior
  - IDE locale setting: VS Code (`agnix.locale`), Neovim plugin, and LSP config bridge
    - Supports explicit null to revert to auto-detection

### Changed
- Documentation and website navigation now include direct install links for VS Code and JetBrains extensions, plus a prominent website link in the README.
- Core: introduce `static_regex!` macro for validated regex initialization (#246)
  - Replaces bare `.unwrap()` on `Regex::new()` with descriptive `.expect()` messages
  - Migrates 36 `OnceLock<Regex>` patterns across 7 files to use the macro
  - Converts `hooks.rs` from `once_cell::sync::Lazy` to `std::sync::OnceLock`
  - Removes `once_cell` direct dependency from agnix-core
  - Adds per-module `test_regex_patterns_compile` tests for all static patterns

### Fixed
- CLI: harden telemetry queue timestamp parsing against malformed data (#231)
  - Replace panic-prone byte-index slicing with safe `str::get()` calls
  - Add ASCII guard, separator validation, and range checks (year, month-aware day bounds, hour, minute, second)
  - Use `checked_sub` for day arithmetic to prevent u32 underflow
- Config validation: accept VER-* prefix in disabled_rules (#233)
- VS Code extension: harden `downloadFile()` cleanup for stream and HTTP failure paths (#240)
  - Closes file/request handles on failure
  - Removes temporary download artifacts on failed downloads
  - Adds regression tests for non-200, stream-error, and success branches
- CLI: gate telemetry module wiring behind `telemetry` feature while preserving command UX via a non-feature stub (#245)
  - `telemetry` module compiles only when feature-enabled
  - Non-feature builds route telemetry calls through `telemetry_stub` no-op facade
  - Added stub-path unit tests and validated both feature and non-feature builds
- LSP backend now uses shared `Arc<String>` document cache entries to avoid full-text cloning on `did_change`, `did_save`, `codeAction`, and `hover` paths (#244)
- LSP now revalidates open documents with bounded concurrency on config changes and drops stale diagnostics from outdated config/content snapshots (#243)
### Security
- ReDoS protection via regex input size limits (MAX_REGEX_INPUT_SIZE = 64KB)
  - Markdown XML tag extraction skips oversized content
  - Cross-platform and prompt engineering validators protected
- File count limits to prevent DoS attacks
  - Default limit of 10,000 files (configurable via max_files_to_validate)
  - CLI flag --max-files to override or disable (--max-files 0)
- Fuzz testing infrastructure with cargo-fuzz
  - Three fuzz targets: fuzz_frontmatter, fuzz_markdown, fuzz_json
  - CI runs 5-minute fuzzing on PRs, 30-minute weekly fuzzing
  - UTF-8 boundary validation for markdown parsing
- Enhanced symlink handling documentation and tests
  - Comprehensive tests for Unix and Windows symlink behavior
  - MAX_SYMLINK_DEPTH = 40 to prevent circular symlink loops
- Security integration test suite (crates/agnix-core/tests/security_integration.rs)
  - Symlink rejection, file size limits, path traversal, file count limits
  - ReDoS protection validation, concurrent validation safety
- Hardened dependency management
  - cargo-audit integration (pinned to v0.21.0) in CI
  - cargo-deny policy with multiple-versions = deny
  - audit.toml and deny.toml configuration files
- Security documentation
  - SECURITY.md with reporting policy and security configuration
  - knowledge-base/SECURITY-MODEL.md with threat model and implementation details
  - Audit history tracking and incident response procedures
- LSP workspace boundary check hardened (#232)
  - Added normalize_path() fallback when canonicalize() fails
  - Prevents path traversal via .. components in non-canonical paths

### Added
- Neovim plugin at `editors/neovim/` with full LSP integration (#187)
  - Automatic LSP attachment to agnix-relevant files
  - Commands: `:AgnixStart`, `:AgnixStop`, `:AgnixRestart`, `:AgnixInfo`, `:AgnixValidateFile`, `:AgnixShowRules`, `:AgnixFixAll`, `:AgnixFixSafe`, `:AgnixIgnoreRule`, `:AgnixShowRuleDoc`
  - Optional Telescope integration for rule browsing
  - `:checkhealth agnix` support
  - Installation via lazy.nvim, packer.nvim, vim-plug, or manual
- Research tracking document (`knowledge-base/RESEARCH-TRACKING.md`) with AI tool inventory and monitoring process (#191)
- Monthly review checklist (`knowledge-base/MONTHLY-REVIEW.md`) with February 2026 review completed (#191)
- Rule contribution and tool support request issue templates (#191)
- Expanded CONTRIBUTING.md with rule authoring guide, evidence requirements, and tier system (#191)
- JetBrains IDE plugin with LSP integration (#196)
  - Supports IntelliJ IDEA, WebStorm, PyCharm, and all JetBrains IDEs (2023.3+)
  - Real-time validation, quick fixes, hover documentation
  - Auto-download of agnix-lsp binary from GitHub releases
  - Settings UI with LSP path configuration, auto-download toggle, trace level
  - Context menu actions: Validate File, Restart Server, Settings
  - Uses LSP4IJ for standard LSP client support
- `agnix schema` command for JSON Schema generation (#206)
  - Outputs JSON Schema for `.agnix.toml` to stdout or file
  - Generated from Rust types using schemars
- Config validation with helpful warnings (#206)
  - Validates `disabled_rules` against known rule ID patterns
  - Validates `tools` array contains recognized tool names
  - Warns on deprecated fields (`mcp_protocol_version`)
- VS Code schema association for `.agnix.toml` autocomplete (#206)
- Opt-in telemetry module with privacy-first design (#209)
  - Disabled by default, requires explicit `agnix telemetry enable`
  - Tracks aggregate metrics: rule trigger counts, error/warning counts, duration
  - Never collects: file paths, contents, user identity
  - Respects DO_NOT_TRACK, CI, GITHUB_ACTIONS environment variables
  - Feature-gated HTTP client for minimal binary size impact
  - Local event queue for offline storage with automatic retry
- `agnix telemetry` subcommand with status/enable/disable commands
- Comprehensive telemetry documentation in SECURITY.md
- Rule ID validation at collection point (defense-in-depth)
- VS Code extension settings UI for configuring all validation options (#225)
  - Settings page accessible via "Open Settings (UI)" command
  - Live preview of all rules with descriptions
  - Changes apply immediately without server restart
  - Built with Svelte for reactive UI

### Changed
- Refactored SkillValidator internal structure for better maintainability (#211)
  - Extracted monolithic 660-line validate() method into ValidationContext struct
  - Grouped validation logic into 11 focused methods by concern
  - Reduced main validate() from ~660 lines to ~78 lines
  - All 128 tests pass without modification (zero behavior changes)
- Refactored HooksValidator into standalone validation functions (#212)
  - Extracted 12 validation rules (CC-HK-001 through CC-HK-012) into standalone functions
  - Reduced main validate() method from ~480 to ~210 lines
  - Organized validation into clear phases with documentation
  - Improved maintainability and testability without changing validation behavior
- Split Hook and Skill validator modules into focused files (#242)
  - Replaced monolithic `rules/hooks.rs` and `rules/skill.rs` with `rules/hooks/{mod,helpers,tests}.rs` and `rules/skill/{mod,helpers,tests}.rs`
  - No validation behavior changes; refactor is layout-only for maintainability

### Fixed
- CLI `--fix` now exits with status `0` when all diagnostics are resolved by auto-fixes (#230)
  - Exit status now reflects post-fix diagnostics for non-dry-run fix modes
  - Added integration regression test for `--fix` success after full auto-fix
- Imports validation now recovers from poisoned shared `ImportCache` locks during project validation (#239)
- Import traversal now revisits files discovered at shallower depth and avoids duplicate REF-001 diagnostics (#239)
### Performance
- Benchmark infrastructure with iai-callgrind for deterministic CI testing (#202)
  - Instruction count benchmarks immune to system load variance
  - Helper script (./scripts/bench.sh) for iai/criterion/bloat workflows
  - Scale testing with 100 and 1000 file projects
  - Memory usage tracking with tracking-allocator
  - CI job blocks merge on performance regressions
  - Cross-platform support (Linux/macOS with Valgrind, Windows uses Criterion only)

## [0.7.2] - 2026-02-05

### Fixed
- npm package wrapper script now preserved during binary installation
  - Fixes "command not found" error when running `agnix` from npm install
  - Postinstall script backs up and restores wrapper script

## [0.7.1] - 2026-02-05

### Fixed
- VS Code extension LSP installation - now downloads LSP-specific archives (`agnix-lsp-*.tar.gz`)
  - Fixes "chmod: No such file or directory" error on macOS ARM64 and Linux ARM64
  - Added binary existence check before chmod for better error messages
- CC-MEM-006 rule now correctly recognizes positive alternatives before negatives
  - Pattern "DO X, don't do Y" now accepted (previously incorrectly flagged)
  - Example: "Fetch web resources fresh, don't rely on cached data" ✓

### Changed
- Release workflow now builds separate LSP archives for VS Code auto-download

## [0.7.0] - 2026-02-05

### Changed
- Refactored LintConfig internal structure for better maintainability (#214)
  - Introduced RuntimeContext struct to group non-serialized state
  - Introduced RuleFilter trait to encapsulate rule filtering logic
  - Public API remains fully backward compatible

### Added
- FileSystem trait for abstracting file system operations (#213)
  - Enables unit testing validators with MockFileSystem instead of requiring real temp files
  - RealFileSystem delegates to std::fs and file_utils for production use
  - MockFileSystem provides HashMap-based in-memory storage with RwLock for thread safety
  - Support for symlink handling and circular symlink detection
  - Integrated into LintConfig via fs() accessor for dependency injection
- Comprehensive test suite for validation rule coverage (#221)
  - Added exhaustive tests for all valid values in enums and constants
  - Improved test coverage for edge cases and error conditions
  - Fixed test logic to properly reflect tool event requirements

### Performance
- Shared import cache at project validation level reduces redundant parsing (#216)

## [0.3.0] - 2026-02-05

### Added
- Comprehensive config file tests (30+ new tests)
- Performance benchmarks for validation pipeline
- Support for partial config files (only specify fields you need)

### Fixed
- Config now allows partial files - users can specify only `disabled_rules` without all other fields
- Windows path false positives - regex patterns (`\n`, `\s`, `\d`) no longer flagged as Windows paths
- Comma-separated tool parsing - both `Read, Grep` and `Read Write` formats now work
- Git ref depth check - `refs/remotes/origin/HEAD` no longer flagged as deep file paths
- Template placeholder links - `{url}`, `{repoUrl}` placeholders skipped in link validation
- Wiki-style links - single-word links like `[[brackets]]` no longer flagged
- CHANGELOG.md excluded from validation (not an agent config file)
- @import/reference false positives - requires file extension for paths with `/`

### Changed
- README updated for v0.3.0 with accurate config examples and benchmark numbers
- Installation now uses `cargo install agnix-cli` from crates.io

## [0.2.0] - 2026-02-05

### Added
- crates.io publishing support (#20)
  - New `agnix-rules` crate for independent rule updates without CLI republish
  - LICENSE-MIT and LICENSE-APACHE files for dual licensing
  - Crate-level READMEs for crates.io pages
  - Automatic crates.io publish on release tags via CI workflow
  - Parity test ensures rules.json stays in sync between knowledge-base and crate
  - Input validation in build.rs for secure code generation
- Language Server Protocol (LSP) implementation for real-time editor validation (#18)
  - New `agnix-lsp` crate with tower-lsp backend
  - Real-time diagnostics on document changes (textDocument/didChange)
  - Real-time diagnostics on file open and save events
  - Quick-fix code actions from Fix objects
  - Hover documentation for frontmatter fields
  - Document content caching for performance
  - Supports all 139 agnix validation rules with severity mapping

  - Workspace boundary validation for security (prevents path traversal)
  - Config caching optimization for performance
  - Editor support for VS Code, Neovim, Helix, and other LSP-compatible editors
  - Comprehensive test coverage with 36 unit and integration tests
  - Installation: `cargo install --path crates/agnix-lsp`
  - LSP now loads `.agnix.toml` from workspace root (#174)
- Multi-tool support via `tools` array in config (#175)
  - Specify `tools = ["claude-code", "cursor"]` to enable only relevant rules
  - Tool-specific rules (CC-*, COP-*, CUR-*) filtered based on tools list
  - Generic rules (AS-*, XP-*, AGM-*, MCP-*, PE-*) always apply
  - Case-insensitive tool name matching
  - Takes precedence over legacy `target` field for flexibility
- VS Code extension with full LSP integration (#22)
  - Real-time diagnostics for all 139 validation rules

  - Status bar indicator showing agnix validation status
  - Syntax highlighting for SKILL.md YAML frontmatter
  - Commands: 'Restart Language Server' and 'Show Output Channel'
  - Configuration: agnix.lspPath, agnix.enable, agnix.trace.server
  - Safe LSP binary detection (prevents command injection)
  - Documentation in editors/vscode/README.md
- Spec Drift Sentinel workflow for automated upstream specification monitoring (#107)
  - Weekly checks for S-tier sources (Agent Skills, MCP, Claude Code, Codex CLI, OpenCode)
  - Monthly checks for A-tier sources (Cursor, GitHub Copilot, Cline)
  - SHA256 content hashing with whitespace normalization for drift detection
  - Baseline storage in `.github/spec-baselines.json`
  - Auto-creates GitHub issues when drift detected with actionable review steps
  - Manual workflow dispatch for on-demand checks and baseline updates
  - Security hardened: HTTPS-only URL validation, SHA-pinned actions, minimal permissions
- Version-aware validation with configurable tool and spec versions
  - New VER-001 rule: Warns when no tool/spec versions are pinned in .agnix.toml
  - Added [tool_versions] section for pinning tool versions (claude_code, codex, cursor, copilot)
  - Added [spec_revisions] section for pinning spec versions (mcp_protocol, agent_skills_spec, agents_md_spec)
  - CC-HK-010 and MCP-008 now add assumption notes when versions are not pinned
  - Diagnostics include assumption field explaining version-dependent behavior
  - Documentation in README.md and VALIDATION-RULES.md
- Cross-layer contradiction detection with 3 new validation rules (XP-004 to XP-006)
  - XP-004: Conflicting build/test commands detection (npm vs pnpm vs yarn vs bun)
  - XP-005: Conflicting tool constraints detection (allow vs disallow across files)
  - XP-006: Multiple instruction layers without documented precedence warning
  - Detects contradictions across CLAUDE.md, AGENTS.md, .cursor/rules, and Copilot files
  - HashMap-based O(n*m) algorithms for efficient conflict detection
  - Word boundary matching to prevent false positives
  - Backup file exclusion (.bak, .old, .tmp, .swp, ~)
- Evidence metadata schema for all 100 validation rules
  - Added `evidence` field to each rule in `knowledge-base/rules.json` with:
    - `source_type`: Classification (spec, vendor_docs, vendor_code, paper, community)
    - `source_urls`: Links to authoritative documentation or specifications
    - `verified_on`: ISO 8601 date of last verification
    - `applies_to`: Tool/version/spec applicability constraints
    - `normative_level`: RFC 2119 level (MUST, SHOULD, BEST_PRACTICE)
    - `tests`: Coverage tracking (unit, fixtures, e2e)
  - Build-time SARIF rule generation from rules.json (replaces hardcoded registry)
  - CI validation tests for evidence metadata completeness and validity
  - Documentation in VALIDATION-RULES.md with schema reference and examples
- Cursor Project Rules support with 6 new validation rules (CUR-001 to CUR-006)
  - CUR-001: Empty .mdc rule file detection
  - CUR-002: Missing frontmatter warning
  - CUR-003: Invalid YAML frontmatter validation
  - CUR-004: Invalid glob pattern in globs field
  - CUR-005: Unknown frontmatter keys warning
  - CUR-006: Legacy .cursorrules migration warning
  - New file type detection for `.cursor/rules/*.mdc` and `.cursorrules`
  - Comprehensive test coverage with 8 fixtures

### Performance
- LSP server now caches ValidatorRegistry in Backend struct (#171)
  - Registry wrapped in Arc and shared across spawn_blocking validation tasks
  - Eliminates redundant HashMap allocations and validator factory lookups per validation
- AS-015 directory size validation now short-circuits when limit exceeded, improving performance on large skill directories (#84)
- Stream file walk to reduce memory usage on large repositories (#172)
  - Replaced collect-then-validate pattern with streaming par_bridge()
  - Eliminated intermediate Vec<PathBuf> storage (O(n) to O(1) memory for file paths)
  - Use AtomicUsize and Arc<Mutex<Vec>> for concurrent metadata collection
  - Small synchronization overhead traded for significant memory reduction on large repos

### Tests
- Added validation pipeline tests for AGENTS.md path collection and files_checked counter (#83)

### Changed
- Tool mappings derived from rules.json at compile time (#176)
  - VALID_TOOLS and TOOL_RULE_PREFIXES now extracted from rules.json evidence metadata
  - New helper functions in agnix-rules: valid_tools(), get_tool_for_prefix(), get_prefixes_for_tool()
  - Config tools array validation uses derived mappings instead of hardcoded list
  - Backward compatibility maintained with "copilot" alias for "github-copilot"
  - Zero runtime cost - all mappings resolved at compile time
- Narrowed agnix-core public API surface (#85)
  - Made `parsers`, `rules`, `schemas`, and `file_utils` modules private
  - Re-exported `Validator` trait for custom validator implementations
  - No breaking changes for agnix-cli or external consumers using documented API

### Removed
- Removed unused config flags `tool_names` and `required_fields` from `.agnix.toml`
  - These flags were never referenced in the codebase
  - Backward compatibility maintained - old configs with these fields still parse correctly

### Fixed
- Mutex locks in streaming validation now use unwrap() for consistent fail-fast on poisoning (#172)
- CLAUDE/AGENTS parity test now resilient to different directory structures (worktrees, symlinks)
  - Replaced brittle `.ancestors().nth(2)` with dynamic workspace root detection
  - New `workspace_root()` helper searches for `[workspace]` in ancestor Cargo.toml files
- JSON output `files_checked` now correctly reports total validated files, not just files with diagnostics
- CLI `--target` flag now validates values instead of silently falling back to "generic"
  - Invalid values rejected with helpful error message showing valid options
  - Prevents configuration typos from going unnoticed
- GitHub Action: Windows binary extension handling (.exe)
- GitHub Action: Missing verbose flag in SARIF output re-run
- GitHub Action: Document jq dependency and fail-on-error input in README
- Config parse errors now display a warning instead of silently falling back to defaults
  - Invalid `.agnix.toml` files show clear error message with parse location
  - Validation continues with default config after displaying warning
  - Warning goes to stderr, preserving JSON/SARIF output validity
- Pinned `cargo-machete` to version `0.9.1` in CI workflow to prevent nondeterministic build failures
- Exclude patterns now prune directories during traversal to reduce IO on large repos
- CLI init command output replaced checkmark emoji with plain text prefix
- Reject `--fix`, `--dry-run`, and `--fix-safe` when using JSON or SARIF output formats
- Exclude glob patterns now match correctly when validate_project() is called with absolute paths (#67)
  - Patterns like `target/**` previously failed to match when walker yielded absolute paths
  - Added path normalization by stripping base path prefix before glob matching
- PE-001 through PE-004 rules now properly dispatch on CLAUDE.md and AGENTS.md files (PromptValidator was implemented but not registered in ValidatorRegistry)
- `is_mcp_revision_pinned()` now correctly returns false when neither `spec_revisions.mcp_protocol` nor `mcp_protocol_version` are explicitly set
  - Previously always returned true due to `serde(default)` on `mcp_protocol_version`
  - This allows MCP-008 assumption notes to appear when no version is configured

### Security
- GitHub Action: Validate version input format to prevent path traversal attacks
- GitHub Action: Sanitize diagnostic messages in workflow commands to prevent injection
- GitHub Action: Use authenticated GitHub API requests when token available (avoids rate limits)
- Blocked @import paths that resolve outside the project root to prevent traversal
- Hardened file reading with symlink rejection and size limits:
  - Added `FileSymlink` error to reject symlinks (prevents path traversal)
  - Added `FileTooBig` error for files exceeding 1 MiB (prevents DoS)
  - New `file_utils` module with `safe_read_file()` using `symlink_metadata()`
  - Applied to validation, imports, fixes, and config loading
  - Cross-platform tests for Unix and Windows symlink handling
- Hardened GitHub Actions workflows with security best practices:
  - Added explicit permissions blocks to all workflows (principle of least privilege)
  - SHA-pinned all third-party actions to prevent supply chain attacks
  - Restricted cache saves to main branch only (prevents cache poisoning from PRs)
  - Documented SHA pin reference in .github/workflows/README.md for maintainability

### Added
- Evaluation harness with `agnix eval` command for measuring rule efficacy
  - Load test cases from YAML manifests with expected rule IDs
  - Calculate precision, recall, and F1 scores per rule and overall
  - Output formats: markdown (default), JSON, CSV
  - Filter by rule prefix (`--filter`)
  - Verbose mode for per-case details (`--verbose`)
  - 39 test cases covering AS-*, CC-SK-*, MCP-*, AGM-*, XP-*, XML-*, REF-* rules
  - Path traversal protection (relative paths only)
  - Documentation in knowledge-base/EVALUATION.md
- MCP-008 rule for protocol version validation with configurable `mcp_protocol_version` option
- 5 new parse error rules with normalized IDs (AS-016, CC-HK-012, CC-AG-007, CC-PL-006, MCP-007)
- Auto-fix support for CC-MEM-005 and CC-MEM-007 memory rules
  - CC-MEM-005: Delete lines containing generic instructions
  - CC-MEM-007: Replace weak constraint language with stronger alternatives
  - CRLF line ending support for correct byte offsets on Windows
- Auto-fix implementations for five additional rules:
  - AS-004: Convert invalid skill names to kebab-case (case-only fixes marked safe)
  - AS-010: Prepend "Use when user wants to " to descriptions missing trigger phrase
  - XML-001: Automatically insert closing XML tags for unclosed elements
  - CC-HK-001: Replace invalid hook event names with closest valid match
  - CC-SK-007: Replace unrestricted Bash access with scoped alternatives (e.g., `Bash(git:*)`)
- Reusable GitHub Action for CI/CD integration:
  - Composite action using pre-built release binaries
  - Inputs for path, strict, target, config, format, verbose, version
  - Outputs for result, errors, warnings, sarif-file
  - GitHub annotations from validation diagnostics
  - Cross-platform support (Linux, macOS, Windows)
  - Test workflow for action validation
- Release workflow for automated binary distribution on version tags:
  - Builds for 5 targets (linux-gnu, linux-musl, macos-x86, macos-arm, windows)
  - Creates archives with SHA256 checksums
  - Extracts release notes from CHANGELOG.md
  - Uploads artifacts to GitHub Releases
- 52 CLI integration tests for comprehensive coverage of all output formats and flags:
  - 12 rule family coverage tests (AS, CC-SK, CC-HK, CC-AG, MCP, XML, CC-PL, COP, AGM, CC-MEM, REF, XP)
  - 5 SARIF output validation tests (schema, tool info, rules, locations, help URIs)
  - 6 text output formatting tests (location, levels, summary, verbose mode)
  - 5 fix/dry-run flag tests (--fix, --fix-safe, --dry-run)
  - 5 flag combination tests (--strict, --verbose, --target, --validate)

- Support for instruction filename variants:
  - CLAUDE.local.md - Claude Code local instructions (not synced to cloud)
  - AGENTS.local.md - Codex CLI/OpenCode local instructions
  - AGENTS.override.md - Codex CLI override file for workspace-specific rules
  - All variants are validated with the same rules as their base files
- Rule parity CI check to ensure documented rules stay in sync with implementation:
  - Added `knowledge-base/rules.json` as machine-readable source of truth for all 84 rules
  - Added `crates/agnix-cli/tests/rule_parity.rs` integration test suite
  - CI fails if rules drift between documentation, SARIF registry, and implementation
  - CLAUDE.md/AGENTS.md updated to document rules.json workflow
- GitHub Copilot instruction files validation with 4 rules (COP-001 to COP-004)
  - COP-001: Empty/missing global copilot-instructions.md
  - COP-002: Invalid YAML frontmatter in scoped instruction files
  - COP-003: Invalid applyTo glob pattern
  - COP-004: Unknown frontmatter keys
  - Supports .github/copilot-instructions.md (global instructions)
  - Supports .github/instructions/*.instructions.md (path-scoped instructions)
  - Config-based copilot category toggle (rules.copilot)
- ValidatorRegistry API for custom validator registration in agnix-core
- AGENTS.md validation rules (AGM-001 to AGM-006)
  - AGM-001: Valid markdown structure
  - AGM-002: Missing section headers
  - AGM-003: Character limit (12000 for Windsurf)
  - AGM-004: Missing project context
  - AGM-005: Unguarded platform features
  - AGM-006: Nested AGENTS.md hierarchy
- AGENTS.md validator now runs via the default registry, with project-level AGM-006 detection
- Explicit HTML anchors in VALIDATION-RULES.md for SARIF help_uri links (#88)
  - Added 80 anchors (one per rule) to fix GitHub anchor mismatch
  - Added tests to validate help_uri format and anchor correctness
- Prompt Engineering validation with 4 rules (PE-001 to PE-004)
  - PE-001: Detects critical content in middle of document (lost in the middle effect)
  - PE-002: Warns when chain-of-thought markers used on simple tasks
  - PE-003: Detects weak imperative language (should, try, consider) in critical sections
  - PE-004: Flags ambiguous instructions (e.g., "be helpful", "as needed")
- PromptValidator implementation in agnix-core
- Config-based prompt_engineering category toggle (rules.prompt_engineering)
- 8 test fixtures in tests/fixtures/prompt/ directory
- 48 comprehensive unit tests for prompt engineering validation
- MCP (Model Context Protocol) validation with 6 rules (MCP-001 to MCP-006)
  - MCP-001: Validates JSON-RPC version is "2.0"
  - MCP-002: Validates required tool fields (name, description, inputSchema)
  - MCP-003: Validates inputSchema is valid JSON Schema
  - MCP-004: Warns when tool description is too short (<10 chars)
  - MCP-005: Warns when tool lacks consent mechanism (requiresApproval/confirmation)
  - MCP-006: Warns about untrusted annotations that should be validated
- McpValidator and McpToolSchema in agnix-core
- Config-based MCP category toggle (rules.mcp)
- 8 test fixtures in tests/fixtures/mcp/ directory
- 48 comprehensive unit tests for MCP validation
- Cross-platform validation rules XP-001, XP-002, XP-003
  - XP-001: Detects Claude-specific features (hooks, context:fork, agent, allowed-tools) in AGENTS.md (error)
    - Supports section guards: Features inside Claude-specific sections (e.g., `## Claude Code Specific`) are allowed
  - XP-002: Validates AGENTS.md markdown structure for cross-platform compatibility (warning)
  - XP-003: Detects hard-coded platform paths (.claude/, .opencode/, .cursor/, etc.) in configs (warning)
- New `cross_platform` config category toggle for XP-* rules
- 5 test fixtures in tests/fixtures/cross_platform/ directory
- 30 comprehensive unit tests for cross-platform validation
- Hook timeout validation rules CC-HK-010 and CC-HK-011
  - CC-HK-010: Warns when hooks lack timeout specification (MEDIUM)
  - CC-HK-011: Errors when timeout value is invalid (negative, zero, or non-integer) (HIGH)
  - Two new test fixtures: no-timeout.json, invalid-timeout.json
- Claude Memory validation rules CC-MEM-004, CC-MEM-006 through CC-MEM-010
  - CC-MEM-004: Validates npm scripts referenced in CLAUDE.md exist in package.json
  - CC-MEM-006: Detects negative instructions ("don't", "never") without positive alternatives
  - CC-MEM-007: Warns about weak constraint language ("should", "try") in critical sections
  - CC-MEM-008: Detects critical content in middle of document (lost in the middle effect)
  - CC-MEM-009: Warns when file exceeds ~1500 tokens, suggests using @imports
  - CC-MEM-010: Detects significant overlap (>40%) between CLAUDE.md and README.md
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
  - CC-SK-005: Validates agent type values (Explore, Plan, general-purpose, or custom kebab-case names 1-64 chars)
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
- Reference validator rules REF-001 and REF-002
  - REF-001: @import references must point to existing files (error)
  - REF-002: Markdown links [text](path) should point to existing files (error)
  - Both rules are in the "imports" category
  - Supports fragment stripping (file.md#section validates file.md)
  - Skips external URLs (http://, https://, mailto:, etc.)
  - 4 test fixtures in tests/fixtures/refs/ directory
  - 31 comprehensive unit tests for reference validation

### Changed
- Removed miette dependency from agnix-core to reduce binary size and compile times
  - agnix-core is now a pure library without terminal output dependencies
  - CLI continues to use colored for output formatting
  - Removed 8 unused LintError variants that used miette-specific features
- Downgraded 5 rules from ERROR to WARNING severity based on RFC 2119 audit:
  - PE-001 (Lost in the middle): Research-based recommendation, not spec violation
  - PE-002 (Chain-of-thought on simple task): Best practice advice, not requirement
  - CC-MEM-004 (Invalid command reference): Helpful validation, not breaking error
  - AGM-003 (Character limit): Uses SHOULD in documentation (Windsurf-specific)
  - AGM-005 (Platform-specific features): Uses SHOULD in documentation
- Imports validator now routes diagnostics by file type:
  - CLAUDE.md files emit CC-MEM-001/002/003 (Claude Code memory rules)
  - Non-CLAUDE markdown files emit REF-001 (generic reference validation)
  - Improved security with path traversal protection (rejects absolute paths)
  - Fixed critical bug: file type now determined per-file during recursion
- XML validator now emits specific rule IDs for each error type:
  - XML-001: Unclosed XML tag
  - XML-002: Mismatched closing tag
  - XML-003: Unmatched closing tag
- Individual XML rules can now be disabled via `disabled_rules` config
- Test fixtures restructured for improved validator integration:
  - Skills: Moved to subdirectory pattern (deep-reference/SKILL.md, missing-frontmatter/SKILL.md, windows-path/SKILL.md)
  - MCP: Renamed with .mcp.json suffix for proper FileType detection
  - Ensures validate_project() correctly identifies fixture types during integration tests
- `validate_project()` now processes files in parallel while maintaining deterministic output
- Directory walking remains sequential, only validation is parallelized
- All validators now respect config-based category toggles and disabled rules
- Config structure enhanced with category-based toggles (legacy flags still supported)
- Knowledge base docs refreshed (rule counts, AGENTS.md support tiers, Cursor rules)
- Fixture layout aligned with detector paths to ensure validators exercise fixtures directly
- CC-HK-010 timeout thresholds now align with official Claude Code documentation
  - Command hooks: warn when timeout > 600s (10-minute default)
  - Prompt hooks: warn when timeout > 30s (30-second default)

### Performance
- Significant speed improvements on projects with many files
- Maintains correctness with deterministic sorting of results
