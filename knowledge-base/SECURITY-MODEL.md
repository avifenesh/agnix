# agnix Security Model

> Threat model, security properties, and implementation details for the agnix linter.

## Overview

agnix is a **local linting tool** that validates agent configuration files. This document describes the security properties agnix provides, known limitations, and mitigation strategies.

## Threat Model

### Trust Boundaries

```
+-------------------+     +------------------+     +-------------------+
|   User Running    |     |   agnix Binary   |     |  Config Files     |
|      agnix        |     |                  |     |  Being Validated  |
|                   |     |  - Parsers       |     |                   |
|   [TRUSTED]       | --> |  - Validators    | <-- |   [UNTRUSTED]     |
|                   |     |  - Fix Engine    |     |                   |
+-------------------+     +------------------+     +-------------------+
                                   |
                                   v
                          +------------------+
                          |   Local File     |
                          |     System       |
                          |                  |
                          |   [TRUSTED]      |
                          +------------------+
```

**Trusted entities:**
- The user running agnix
- The local filesystem (for reading/writing)
- The agnix binary itself

**Untrusted entities:**
- Configuration files being validated (may be malicious)
- File paths within configuration files (imports, references)

### Out of Scope Threats

The following are explicitly NOT addressed:
- **Network attacks**: agnix makes no network requests
- **Privilege escalation**: agnix runs with user privileges only
- **Code execution**: agnix does not execute external commands from config files
- **Supply chain beyond deps**: We audit dependencies, not the entire Rust toolchain

## Security Properties

### 1. Symlink Rejection

**Property**: agnix never follows symbolic links when reading files.

**Implementation**: `crates/agnix-core/src/file_utils.rs:safe_read_file_with_limit()`

```rust
pub fn safe_read_file(path: &Path) -> LintResult<String> {
    let metadata = std::fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() {
        return Err(LintError::FileSymlink { path: path.to_path_buf() });
    }
    // ...
}
```

**Rationale**: Prevents path traversal attacks where a symlink could redirect reads to sensitive files outside the project (e.g., `/etc/passwd`).

**Limitation**: TOCTOU window exists between `symlink_metadata()` check and file read.

**Platform-Specific Atomic Alternatives**: On Unix, `open()` with `O_NOFOLLOW` flag provides atomic symlink rejection. On Windows, `CreateFile()` with `FILE_FLAG_OPEN_REPARSE_POINT` provides similar protection. These would eliminate the TOCTOU window but require platform-specific code. The current cross-platform implementation using `symlink_metadata()` provides equivalent security for the linter's threat model.

### 2. File Size Limits

**Property**: No single file larger than 1 MiB is processed.

**Implementation**: `crates/agnix-core/src/file_utils.rs:safe_read_file_with_limit()`

```rust
pub const DEFAULT_MAX_FILE_SIZE: u64 = 1024 * 1024; // 1 MiB

// In safe_read_file_with_limit():
let metadata = std::fs::symlink_metadata(path)?;
if metadata.len() > DEFAULT_MAX_FILE_SIZE {
    return Err(LintError::FileTooBig { path: path.to_path_buf(), size: metadata.len() });
}
```

**Rationale**: Prevents memory exhaustion from processing extremely large files.

### 3. File Count Limits

**Property**: Validation stops after processing a configurable maximum number of files.

**Implementation**: `crates/agnix-core/src/pipeline.rs:validate_project_with_registry()`, `crates/agnix-core/src/config.rs:274-312`

```rust
// Default limit
pub const DEFAULT_MAX_FILES: usize = 10_000;

// Atomic counter during parallel walk - only counts lintable files
if file_type != FileType::Unknown {
    let count = files_checked.fetch_add(1, Ordering::SeqCst);
    if let Some(limit) = max_files {
        if count >= limit {
            limit_exceeded.store(true, Ordering::SeqCst);
            return Vec::new();
        }
    }
}
```

**Rationale**: Prevents DoS via projects with millions of small files.

**Configuration**:
```toml
# .agnix.toml
max_files_to_validate = 10000  # Default
```

```bash
# CLI
agnix --max-files 5000 .
agnix --max-files 0 .  # Disable (not recommended)
```

### 4. Regex Input Size Limits

**Property**: Regex operations are skipped for content exceeding 64KB.

**Implementation**: `crates/agnix-core/src/parsers/markdown.rs:17-53`

```rust
pub const MAX_REGEX_INPUT_SIZE: usize = 65536; // 64KB

pub fn extract_xml_tags(content: &str) -> Vec<XmlTag> {
    // Security: Skip regex processing for oversized content
    if content.len() > MAX_REGEX_INPUT_SIZE {
        return Vec::new();
    }
    // ...
}
```

**Rationale**: Prevents ReDoS (Regular Expression Denial of Service) attacks where crafted input causes catastrophic backtracking.

### 5. Path Traversal Detection

**Property**: Import paths attempting to escape the project root are detected.

**Implementation**: `crates/agnix-core/src/rules/imports.rs:184-201`

```rust
pub fn normalize_join(base: &Path, relative: &str) -> Option<PathBuf> {
    // Detect escape attempts
    let normalized = base.join(relative);
    if !normalized.starts_with(base) {
        return None; // Path escapes base
    }
    Some(normalized)
}
```

**Rationale**: Prevents config files from referencing files outside the project.

### 6. Atomic File Writes

**Property**: Fix application uses atomic temp-file-then-rename.

**Implementation**: `crates/agnix-core/src/file_utils.rs:75-95`

```rust
pub fn safe_write_file(path: &Path, content: &str) -> LintResult<()> {
    // Write to temp file first
    let temp_path = path.with_extension("tmp");
    std::fs::write(&temp_path, content)?;
    // Atomic rename
    std::fs::rename(&temp_path, path)?;
    Ok(())
}
```

**Rationale**: Prevents partial writes that could corrupt files.

### 7. Symlink Depth Limiting

**Property**: Directory traversal limits symlink resolution depth.

**Implementation**: `crates/agnix-core/src/fs.rs:304-343`

```rust
impl MockFileSystem {
    pub const MAX_SYMLINK_DEPTH: u32 = 40;

    fn metadata_with_depth(&self, path: &Path, depth: u32) -> io::Result<FileMetadata> {
        if depth > Self::MAX_SYMLINK_DEPTH {
            return Err(io::Error::other("too many levels of symbolic links"));
        }
        // ...
    }
}
```

**Rationale**: Prevents infinite loops from circular symlinks.

## Known Limitations

### 1. TOCTOU (Time-of-Check-Time-of-Use)

**Issue**: Gap exists between checking file properties and reading content.

```
Time T1: is_symlink() returns false
Time T2: Attacker replaces file with symlink
Time T3: File read follows the new symlink
```

**Impact**: Low - requires local filesystem access and precise timing.

**Mitigation**: Accept the risk for a linter; high-security environments should use sandboxing.

### 2. Platform Differences

**Issue**: Symlink behavior varies between Unix and Windows.

- Unix: Full symlink support, tests comprehensive
- Windows: Limited symlink support, fewer tests

**Impact**: Medium - security properties may not hold on all platforms.

**Mitigation**: Document differences, test on CI for both platforms.

### 3. YAML Complexity

**Issue**: Deeply nested YAML can cause high memory usage within size limits.

**Impact**: Low - bounded by 1 MiB file size.

**Mitigation**: Parser library handles depth limits; consider adding explicit depth check.

### 4. Parser Bugs

**Issue**: Parsers may have undiscovered bugs despite testing.

**Impact**: Variable - could range from crashes to security issues.

**Mitigation**:
- Property-based testing (proptest)
- Fuzz testing (cargo-fuzz)
- Unit tests for edge cases

## Testing Strategy

### Property-Based Testing (Proptest)

Located in parser modules, tests invariants:
- Parsers never panic on any input
- Output byte offsets within input bounds
- Valid structures produced

#### Verification Method

Run with:
```bash
cargo test --lib
```

Property tests generate thousands of random inputs to verify invariants hold.

### Fuzz Testing (cargo-fuzz)

Located in `crates/agnix-core/fuzz/`:
- `fuzz_frontmatter.rs`: YAML frontmatter parsing
- `fuzz_markdown.rs`: Markdown import/XML extraction (with UTF-8 boundary validation)
- `fuzz_json.rs`: JSON config parsing

#### Verification Method

Run locally:
```bash
cd crates/agnix-core
cargo +nightly fuzz run fuzz_markdown -- -max_total_time=300 -max_len=131072
```

Fuzzing runs continuously on CI for 5 minutes per PR, 30 minutes weekly.

### Security Integration Tests

Located in `crates/agnix-core/tests/security_integration.rs`:
- Symlink rejection tests
- File size limit tests (1 MiB boundary)
- Path traversal tests (with explicit assertions)
- File count limit tests (including concurrent validation)
- ReDoS protection tests (oversized input handling)
- Large input handling

#### Verification Method

Run with:
```bash
cargo test --test security_integration
```

### Unit Tests for Security Boundaries

Security-critical code paths have dedicated unit tests:
- `file_utils.rs`: Symlink rejection, size limits, TOCTOU documentation
- `fs.rs`: MAX_SYMLINK_DEPTH boundary tests (40 links exactly, 41 links fail)
- `cross_platform.rs`: ReDoS protection for regex operations
- `prompt.rs`: ReDoS protection for prompt engineering rules
- `fixes.rs`: UTF-8 boundary validation before string operations

#### Verification Method

Run with:
```bash
cargo test --lib
```

### CI Security Checks

Defined in `.github/workflows/security.yml`:
- `cargo audit`: Known vulnerability scanning
- `cargo deny`: License and duplicate checking (multiple-versions = deny)
- CodeQL: Static analysis (security-extended queries)

## Incident Response

### Reporting

1. Do NOT open public GitHub issues for vulnerabilities
2. Email: aviarchi1994@gmail.com (subject: SECURITY)
3. Include reproduction steps

### Response Timeline

- Acknowledgment: 48 hours
- Status update: 7 days
- Fix release: As soon as practical

### Disclosure Policy

1. Vulnerability confirmed
2. Fix developed and tested
3. New version released
4. Advisory published (GitHub Security Advisory)
5. Credit to reporter (unless anonymous requested)

## Configuration Reference

### Security-Relevant Options

| Option | Default | Description |
|--------|---------|-------------|
| `max_files_to_validate` | 10,000 | Maximum files before stopping |
| `exclude` | node_modules, .git, target | Directories to skip |

### CLI Flags

| Flag | Description |
|------|-------------|
| `--max-files N` | Override file limit |
| `--max-files 0` | Disable limit (not recommended) |

## Audit History

| Date | Scope | Findings |
|------|-------|----------|
| 2026-02-05 | Initial security hardening | ReDoS protection, file limits, fuzz testing |

## References

- [SECURITY.md](../SECURITY.md) - Security policy and reporting
- [file_utils.rs](../crates/agnix-core/src/file_utils.rs) - Core file I/O security
- [fs.rs](../crates/agnix-core/src/fs.rs) - FileSystem trait with symlink handling
- [security_integration.rs](../crates/agnix-core/tests/security_integration.rs) - Security tests
