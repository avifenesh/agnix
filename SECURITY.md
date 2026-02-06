# Security Policy

## Reporting Security Vulnerabilities

If you discover a security vulnerability in agnix, please report it responsibly:

1. **Do NOT open a public issue** for security vulnerabilities
2. Email the maintainer directly at: aviarchi1994@gmail.com
3. Include:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested fix (if any)

You can expect:

- Acknowledgment within 48 hours
- Status update within 7 days
- Credit in release notes (unless you prefer anonymity)

## Supported Versions

| Version | Supported |
| ------- | --------- |
| 0.7.x   | Yes       |
| < 0.7   | No        |

## Security Model

Agnix is a **local linting tool** that validates agent configuration files. Its threat model assumes:

- **Trusted input files**: Files being validated are from the user's own codebase
- **Local execution**: The tool runs locally, not as a service
- **Opt-in telemetry only**: Network submission is disabled by default and requires explicit opt-in (see Telemetry section)

For detailed security architecture, threat model, and implementation details, see [SECURITY-MODEL.md](knowledge-base/SECURITY-MODEL.md).

### Security Measures

| Feature                      | Description                                              | Default   |
| ---------------------------- | -------------------------------------------------------- | --------- |
| **Symlink Rejection**        | All file reads reject symlinks to prevent path traversal | Always on |
| **File Size Limits**         | Maximum file size to prevent memory exhaustion           | 1 MiB     |
| **File Count Limits**        | Maximum files to validate to prevent DoS                 | 10,000    |
| **Regex Input Limits**       | Maximum input to regex operations to prevent ReDoS       | 64 KB     |
| **Path Traversal Detection** | Import validation detects `../` escape attempts          | Always on |
| **Atomic Writes**            | Fix application uses atomic temp-file-then-rename        | Always on |
| **No Command Execution**     | agnix does not execute external commands or scripts      | N/A       |

### Known Limitations

1. **TOCTOU Window**: There is a time-of-check-time-of-use gap between checking file properties and reading content. An attacker with local filesystem access could potentially exploit this, but impact is limited to reading unexpected content. Platform-specific atomic alternatives (O_NOFOLLOW on Unix) would eliminate this but are not currently implemented.

2. **Platform Differences**: Symlink behavior varies between Unix and Windows. Tests are primarily validated on Unix.

3. **YAML Complexity**: While file size limits provide basic protection, deeply nested YAML structures could cause high memory usage within the 1 MiB limit.

4. **Parser Bugs**: While we use fuzz testing and property-based testing, parsers may have undiscovered bugs.

5. **Telemetry ID Generation**: If telemetry is enabled (disabled by default), session IDs are generated using `uuid::Uuid::new_v4()` which uses the operating system's random number generator. No personally identifiable information is collected.

## Security Configuration

### .agnix.toml Options

```toml
# Maximum files to validate (DoS protection)
# Default: 10,000. Set to None to disable (not recommended)
max_files_to_validate = 10000

# Exclude patterns (skip untrusted directories)
exclude = [
    "node_modules/**",
    ".git/**",
    "target/**",
]
```

### CLI Options

```bash
# Override file limit
agnix --max-files 5000 .

# Disable file limit (not recommended)
agnix --max-files 0 .
```

## Supply Chain Security

We use multiple layers of dependency verification:

1. **cargo-audit**: Checks for known CVEs in dependencies (runs on every PR)
2. **cargo-deny**: Checks licenses, duplicates, and sources
3. **Dependabot**: Automatic security updates
4. **CodeQL**: Static analysis for Rust code

## Safe Error Handling Patterns

The codebase follows these patterns to maintain security:

1. **Graceful Degradation**: Parsing errors skip the problematic file rather than crashing
2. **No Sensitive Data in Errors**: Error messages avoid exposing file contents
3. **UTF-8 Boundary Safety**: Fix application validates UTF-8 character boundaries
4. **Bounded Iteration**: Regex matches and file walks use limits to prevent exhaustion
5. **Early Validation**: Invalid inputs are rejected at parsing stage

## Telemetry

agnix includes **opt-in** telemetry to help improve the tool. Telemetry is disabled by default.

### Privacy Guarantees

When telemetry is enabled, we collect only aggregate statistics:

**What we collect:**

- File type counts (e.g., "5 skills, 2 MCP configs") - NOT file paths or names
- Rule trigger counts (e.g., "AS-001: 3 times") - NOT diagnostic messages
- Error/warning/info counts
- Validation duration
- Random installation ID (UUIDv4, not tied to user identity)

**What we NEVER collect:**

- File paths or directory structure
- File contents or code
- User identity, email, or system information
- IP addresses (telemetry server does not log IPs)

### Environment-Aware Disable

Telemetry is automatically disabled in:

- CI environments (CI, GITHUB_ACTIONS, GITLAB_CI, TRAVIS, etc.)
- When DO_NOT_TRACK environment variable is set (any value)
- When AGNIX_TELEMETRY=0 or AGNIX_TELEMETRY=false

### Controlling Telemetry

```bash
# Check current status
agnix telemetry status

# Enable telemetry (opt-in)
agnix telemetry enable

# Disable telemetry
agnix telemetry disable
```

### Data Storage

- Config: `~/.config/agnix/telemetry.json` (or platform equivalent)
- Queue: `~/.local/share/agnix/telemetry_queue.json` (for offline storage)

### Compile-Time Feature Gate

Telemetry HTTP submission is also gated by a Cargo feature. By default, events are only stored locally. To enable HTTP submission:

```bash
cargo install agnix-cli --features telemetry
```

## Security Updates

Security fixes are released as patch versions (e.g., 0.8.0 -> 0.8.1) and announced in:

- GitHub Releases
- CHANGELOG.md

## Contact

- Security issues: aviarchi1994@gmail.com
- General issues: GitHub Issues
- Repository: https://github.com/avifenesh/agnix
