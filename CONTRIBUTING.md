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

Each rule in `rules.json` must include complete `evidence` metadata:

```json
{
  "id": "XX-001",
  "message": "...",
  "evidence": {
    "source_type": "spec|research|implementation",
    "source_urls": ["..."],
    "verified_on": "2026-01-01",
    "applies_to": ["claude-code"],
    "normative_level": "must|should|may",
    "tests": ["test_name"]
  }
}
```

## Pull Request Process

1. **Update CHANGELOG.md** - Required for all PRs (skip with `[skip changelog]` in title)
2. **Add tests** - Every feature/fix must have tests
3. **Wait for CI** - The claude workflow is the major quality gate
4. **Get review approval** - At least one approval required

## Commit Messages

Use conventional commits:

- `feat:` - New feature
- `fix:` - Bug fix
- `docs:` - Documentation
- `refactor:` - Code refactoring
- `test:` - Tests
- `chore:` - Maintenance

Reference issues when applicable: `fix: resolve timeout issue (#123)`

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
editors/
  vscode/         # VS Code extension
knowledge-base/   # Rules documentation
tests/fixtures/   # Test cases
```

## Questions?

Open an issue or start a discussion.
