# Contributing to agnix

Thank you for contributing to agnix.

## Development Setup

```bash
git clone https://github.com/avifenesh/agnix
cd agnix
cargo build
cargo test
```

## Code Style

Before committing:

```bash
cargo fmt
cargo clippy --all-targets
```

## Adding a New Rule

1. **Add to rules.json** - `knowledge-base/rules.json` is the source of truth
2. **Add to VALIDATION-RULES.md** - `knowledge-base/VALIDATION-RULES.md` for human docs
3. **Implement validator** - `crates/agnix-core/src/rules/`
4. **Add test fixtures** - `tests/fixtures/`
5. **Run parity tests** - CI enforces rules.json and VALIDATION-RULES.md stay in sync

When editing project memory instructions, keep `CLAUDE.md` and `AGENTS.md` byte-identical.

Each rule in `rules.json` must include complete `evidence` metadata. See [Rule Evidence Requirements](#rule-evidence-requirements) below for field details.

## Rule Evidence Requirements

Each rule in `knowledge-base/rules.json` must include an `evidence` object documenting its authoritative source. The evidence fields are:

| Field | Type | Description |
|-------|------|-------------|
| `source_type` | enum | Classification of the source: `spec` (official specification), `vendor_docs` (vendor documentation), `vendor_code` (vendor source code), `paper` (academic research), `community` (community research such as awesome-slash) |
| `source_urls` | string[] | One or more URLs pointing to the authoritative documentation, specification, or research paper that supports this rule |
| `verified_on` | string | ISO 8601 date (YYYY-MM-DD) when the source was last verified to be current |
| `applies_to` | object | Applicability constraints: `tool` (specific tool name, e.g., "claude-code"), `version_range` (semver range, e.g., ">=1.0.0"), `spec_revision` (spec version date). Empty object `{}` means the rule applies universally |
| `normative_level` | enum | RFC 2119 level indicating rule strength: `MUST` (spec violation), `SHOULD` (strong recommendation), `BEST_PRACTICE` (advisory) |
| `tests` | object | Test coverage tracking: `{ "unit": true/false, "fixtures": true/false, "e2e": true/false }` |

See `knowledge-base/VALIDATION-RULES.md` for the full evidence schema reference with examples.

## Rule ID Conventions

Rule IDs follow the format `[PREFIX]-[NUMBER]` where the prefix indicates the category:

| Prefix | Category | Example |
|--------|----------|---------|
| `AS-` | Agent Skills | AS-001 through AS-016 |
| `CC-SK-` | Claude Code Skills | CC-SK-001 through CC-SK-009 |
| `CC-HK-` | Claude Code Hooks | CC-HK-001 through CC-HK-012 |
| `CC-MEM-` | Claude Code Memory | CC-MEM-001 through CC-MEM-010 |
| `CC-AG-` | Claude Code Agents | CC-AG-001 through CC-AG-007 |
| `CC-PL-` | Claude Code Plugins | CC-PL-001 through CC-PL-006 |
| `MCP-` | Model Context Protocol | MCP-001 through MCP-008 |
| `CUR-` | Cursor | CUR-001 through CUR-006 |
| `COP-` | GitHub Copilot | COP-001 through COP-006 |
| `AGM-` | AGENTS.md | AGM-001 through AGM-006 |
| `XP-` | Cross-Platform | XP-001 through XP-006 |
| `PE-` | Prompt Engineering | PE-001 through PE-004 |
| `XML-` | XML Validation | XML-001 through XML-003 |
| `REF-` | Reference/Import Validation | REF-001 through REF-002 |
| `VER-` | Version Awareness | VER-001 |

To find the next available number for a prefix, check `knowledge-base/rules.json` for the highest existing number in that prefix group and increment by one.

## Implementing a Validator

Step-by-step process for adding a new validation rule:

1. **Add the rule to `knowledge-base/rules.json`** -- Include all required fields: `id`, `name`, `severity`, `category`, `message`, `detection`, `fix`, and complete `evidence` metadata. The `crates/agnix-rules/rules.json` file is automatically synchronized during the build process.

2. **Add documentation to `knowledge-base/VALIDATION-RULES.md`** -- Document the rule following the existing format with detection logic, fix description, and source citation. CI parity tests will fail if the rule exists in one file but not the other.

3. **Implement the `Validator` trait** -- Add validation logic in `crates/agnix-core/src/rules/`. Look at existing validators for patterns:
   - `xml_balance.rs` -- simple single-file validator
   - `agents_md.rs` -- project-level validator with cross-file analysis
   - `skill/mod.rs` and `hooks/mod.rs` -- complex validators split into focused `helpers.rs` and `tests.rs` modules

4. **Register in `ValidatorRegistry`** -- Add the validator factory to the `DEFAULTS` constant in `crates/agnix-core/src/registry.rs`. It will be included automatically via `ValidatorRegistry::with_defaults()`. External validators can use `ValidatorProvider` trait instead.

5. **Add test fixtures** -- Create test files in `tests/fixtures/` matching the validator's expected file type detection patterns. Fixtures should cover both valid and invalid configs.

6. **Run tests** -- Verify everything passes:
   ```bash
   cargo test                              # Full test suite
   cargo test -p agnix-rules --test parity # Parity check
   cargo test -p agnix-core                # Core validator tests
   ```

## Testing Requirements

All new rules must include:

- **Unit tests** in the validator module (test individual rule detection and edge cases)
- **Integration tests** via test fixtures in `tests/fixtures/` (test end-to-end validation)
- **Parity tests** pass (rules.json matches VALIDATION-RULES.md; rules.json matches crates/agnix-rules/rules.json)
- **Full test suite** passes before submitting a PR (`cargo test`)

## Tool Tier System

agnix organizes tool support into tiers based on community adoption and maintenance commitment:

| Tier | Policy | Testing Requirement |
|------|--------|---------------------|
| **S** | Test always | Every CI run validates against these tools |
| **A** | Test on major changes | Tested when changes affect tool-specific rules |
| **B** | Test on significant changes if time permits | Spot-tested on large changes |
| **C** | Community reports fixes only | Fixes accepted via community issues |
| **D** | No active support, nice to have | Can try once in a while, mainly if users request |
| **E** | No support, community contributions only | Full community support and contributions |

Current tier assignments are documented in [`knowledge-base/RESEARCH-TRACKING.md`](./knowledge-base/RESEARCH-TRACKING.md). When proposing a tier change, open a GitHub issue with adoption data to support the change.

## Community Feedback

We welcome community input through several channels:

- **GitHub Issues** -- Use the issue templates for structured feedback:
  - [Bug Report](.github/ISSUE_TEMPLATE/bug_report.md) -- Report validation errors
  - [Feature Request](.github/ISSUE_TEMPLATE/feature_request.md) -- Suggest new capabilities
  - [Rule Contribution](.github/ISSUE_TEMPLATE/rule_contribution.md) -- Propose new validation rules
  - [Tool Support Request](.github/ISSUE_TEMPLATE/tool_support_request.md) -- Request support for new tools
- **GitHub Discussions** -- General questions, ideas, and community discussion

## Pull Request Process

1. **Update CHANGELOG.md** - Required for all PRs (skip with `[skip changelog]` in title)
2. **Add tests** - Every feature/fix must have tests
3. **Wait for CI** - The claude workflow is the major quality gate
4. **Get review approval** - At least one approval required

## Backward-Compatibility Policy

agnix follows a stability policy to protect downstream consumers (CLI, LSP, MCP, editor extensions) from accidental API breakage.

### Stability Tiers

| Tier | Scope | Contract |
|------|-------|----------|
| **Public/Stable** | Re-exported types at `agnix_core` root (`LintConfig`, `Diagnostic`, `DiagnosticLevel`, `Fix`, `LintError`, `LintResult`, `ValidationResult`, `FileType`, `ValidatorRegistry`, `ValidatorFactory`, `Validator` trait, `FileSystem` trait, `MockFileSystem`, `RealFileSystem`, `FixResult`, `ConfigWarning`, `FilesConfig`, `generate_schema`) and all `agnix_rules` public items (`RULES_DATA`, `VALID_TOOLS`, `TOOL_RULE_PREFIXES`, `rule_count`, `get_rule_name`, `valid_tools`, `normalize_tool_name`, etc.) | Breaking changes require a minor version bump (pre-1.0) or major version bump (post-1.0) with advance notice in CHANGELOG.md |
| **Public/Unstable** | Accessible public modules that may change between minor versions: `authoring`, `eval`, `i18n`, `validation` | May change with a minor version bump; consumers should pin exact versions if depending on these |
| **Internal** | Private modules not accessible outside the crate: `parsers`, `rules`, `schemas`, `file_utils`, `regex_util`, `span_utils` | May change freely in any release |

### What Constitutes a Breaking Change

The following changes to **Public/Stable** items are considered breaking:

- Removing a public type, function, or constant
- Changing a public function signature (parameter types, return type)
- Changing a struct field type or removing a field
- Removing an enum variant
- Changing a trait method signature or adding a required method without a default

### What is Non-Breaking

These changes are safe to make in any release:

- Adding new enum variants (this may break exhaustive matches; consumers should use wildcard `_` arms to stay forward-compatible)
- Adding new optional struct fields with `#[serde(default)]`
- Adding new public functions, types, or modules
- Adding new validators to `ValidatorRegistry::with_defaults()`
- Adding new rules to `agnix_rules`
- Adding new trait methods with default implementations

### Feature Flags

agnix-core intentionally has no feature flags. It is a focused validation library with no optional heavyweight dependencies. Only the CLI crate (`agnix-cli`) has a `telemetry` feature flag for opt-in analytics.

When to add feature flags in the future:
- If a new dependency adds significant compile time or binary size
- If a feature is truly optional and not needed by most consumers

### Pre-1.0 Caveat

While agnix is below version 1.0, this policy is followed in good faith. Minor versions may occasionally contain breaking changes when necessary, but these will always be documented in CHANGELOG.md with migration instructions.

## Commit Messages

Use conventional commits:

- `feat:` - New feature
- `fix:` - Bug fix
- `docs:` - Documentation
- `refactor:` - Code refactoring
- `test:` - Tests
- `chore:` - Maintenance

Reference issues when applicable: `fix: resolve timeout issue (#123)`

## Contributing Translations

agnix supports multiple languages. See [docs/TRANSLATING.md](docs/TRANSLATING.md) for:
- Adding new locales
- Translation guidelines
- Testing translations

## Running Tests

```bash
# All tests
cargo test

# Specific crate
cargo test -p agnix-core

# With output
cargo test -- --nocapture
```

### Security Tests

```bash
# Security integration tests
cargo test --test security_integration

# Fuzz testing (requires nightly)
cd crates/agnix-core
cargo +nightly fuzz run fuzz_markdown -- -max_total_time=300
cargo +nightly fuzz run fuzz_frontmatter -- -max_total_time=300
cargo +nightly fuzz run fuzz_json -- -max_total_time=300

# Dependency audit
cargo audit
cargo deny check
```

## Project Structure

```
crates/
  agnix-rules/    # Rule definitions (generated)
  agnix-core/     # Validation engine
  agnix-cli/      # CLI binary
  agnix-lsp/      # Language server
  agnix-mcp/      # MCP server
editors/
  neovim/         # Neovim extension
  vscode/         # VS Code extension
  jetbrains/      # JetBrains extension scaffold
knowledge-base/   # Rules documentation
scripts/          # Development automation scripts
website/          # Docusaurus documentation website
tests/fixtures/   # Test cases
```

## Questions?

Open an issue or start a discussion.
