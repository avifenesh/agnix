# RUSTSEC Advisory Tracking

This document tracks RUSTSEC security advisories that are currently ignored in the project and explains why they are ignored and when they should be reviewed.

Related: [Issue #346](https://github.com/avifenesh/agnix/issues/346) (this tracking system resolves that issue)

## Currently Ignored Advisories

### RUSTSEC-2024-0384 — `instant` (via `notify`)

**Status**: Waiting for `notify` 7.0 release

**Details**:
- The `instant` crate is unmaintained but functionally correct
- It's pulled in as a transitive dependency via `notify`
- The `notify` project plans to drop `instant` in version 7.0

**Risk Level**: Low
- The crate is unmaintained, but there are no known security vulnerabilities
- Functionality is stable and correct
- Not exposed in public APIs

**Action Items**:
- Monitor `notify` releases for version 7.0
- Once `notify` 7.0 is available, update the dependency
- Remove this advisory ignore from:
  - `deny.toml` in the `[advisories]` ignore list
  - `.github/workflows/security.yml` in the `cargo audit` command

**References**:
- Advisory: https://rustsec.org/advisories/RUSTSEC-2024-0384
- notify issue tracker: https://github.com/notify-rs/notify

---

### RUSTSEC-2025-0141 — `bincode` (via `iai-callgrind`)

**Status**: Dev-only dependency used for benchmarks

**Details**:
- The `bincode` crate has a security advisory
- It's only used via `iai-callgrind`, which is a dev dependency for benchmarks
- Not included in release binaries

**Risk Level**: Low
- Dev-only dependency (not in production code)
- Used only for benchmark serialization
- Not exposed to untrusted input

**Action Items**:
- Monitor `iai-callgrind` updates for a version that uses a patched `bincode`
- Check periodically if `iai-callgrind` has switched to a different serialization library
- Remove this advisory ignore from:
  - `deny.toml` in the `[advisories]` ignore list
  - `.github/workflows/security.yml` in the `cargo audit` command

**References**:
- Advisory: https://rustsec.org/advisories/RUSTSEC-2025-0141
- iai-callgrind repository: https://github.com/iai-callgrind/iai-callgrind

---

## Review Schedule

These advisories should be reviewed:

1. **Before each release** as part of the [Pre-release Checks](RELEASING.md#pre-release-checks) (highest priority)
2. **When running `cargo update`** to check if dependencies have been updated (opportunistic)
3. **Monthly** as part of the [Monthly Review](../knowledge-base/MONTHLY-REVIEW.md) process (regular cadence)

### Review Process

To review these advisories:

```bash
# Update dependencies
cargo update

# Run cargo audit without ignores to see current status
cargo audit

# Run cargo deny to check advisories (validates deny.toml ignores)
cargo deny check advisories

# Check if any of the ignored advisories have been resolved
cargo tree -i instant -e normal      # Check if notify still depends on instant (normal deps only)
cargo tree -i bincode -e dev         # Check if iai-callgrind still depends on bincode (dev deps)

# If a dependency has been updated and no longer triggers the advisory:
# 1. Remove the advisory ID from deny.toml [advisories] ignore list
# 2. Remove the --ignore flag from .github/workflows/security.yml
# 3. Update this document to mark the advisory as resolved
# 4. Close or update the related tracking issue
```

## Adding New Advisory Ignores

If a new advisory needs to be temporarily ignored:

1. **Document the reason** in this file with:
   - Advisory ID and affected crate
   - Why it's being ignored (waiting for upstream fix, low risk, etc.)
   - Risk assessment
   - Clear action items for removal
   - Reference links

2. **Update `deny.toml`**:
   - Add the advisory ID to the `[advisories] ignore` list
   - Add an inline comment explaining the ignore

3. **Update CI**:
   - Add `--ignore RUSTSEC-YYYY-NNNN` to the `cargo audit` command in `.github/workflows/security.yml`

4. **Create or update tracking issue** with the advisory details

5. **Set a reminder** to review the advisory in the next monthly review

### Template for New Advisory

Copy this template when adding a new ignored advisory:

```markdown
### RUSTSEC-YYYY-NNNN — `crate-name` (via `parent-crate`)

**Status**: [One sentence describing current state]

**Details**:
- [Why is this advisory triggered?]
- [What is the dependency chain?]
- [What is the plan to resolve?]

**Risk Level**: [High/Medium/Low]
- [Justify the risk level]
- [Describe exposure/impact]
- [Note any mitigations]

**Action Items**:
- [What should be monitored?]
- [What triggers removal?]
- [Where to remove the ignore?]

**References**:
- Advisory: https://rustsec.org/advisories/RUSTSEC-YYYY-NNNN
- Upstream tracker: [link]
```

## Future Automation

The review process could be partially automated:
- A scheduled CI job could run `cargo tree -i instant -e normal` and `cargo tree -i bincode -e dev` weekly
- Results could be posted as a comment on the tracking issue
- Manual review would still be required to decide when to remove ignores

## Resolved Advisories

_(None yet - this section will track advisories that have been resolved)_
