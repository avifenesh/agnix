# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

If you discover a security vulnerability in agnix, please report it responsibly:

1. **Do NOT open a public issue** for security vulnerabilities
2. Email the maintainer directly at: avifenesh@gmail.com
3. Include:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested fix (if any)

You can expect:
- Acknowledgment within 48 hours
- Status update within 7 days
- Credit in release notes (unless you prefer anonymity)

## Security Model

agnix is a **local linting tool** that validates agent configuration files. Its threat model assumes:

- **Trusted input files**: Files being validated are from the user's own codebase
- **Local execution**: The tool runs locally, not as a service
- **Opt-in telemetry only**: Network access is disabled by default (see Telemetry section)

### Security Measures

1. **File Size Limits**: All file reads are capped at 1 MiB to prevent resource exhaustion
2. **Symlink Rejection**: Symbolic links are rejected to prevent path traversal attacks
3. **Path Validation**: The LSP server validates files are within workspace boundaries
4. **No Command Execution**: agnix does not execute external commands or scripts
5. **Safe File Writes**: Atomic writes with symlink checks for fix application

### Dependency Security

We use `cargo-audit` in CI to check for known vulnerabilities in dependencies. The security workflow runs:
- On every push to main
- On every pull request
- Weekly on schedule

### Known Limitations

- **TOCTOU Window**: A small time-of-check-time-of-use window exists between file validation and read/write operations. This is acceptable for the threat model (trusted local files).
- **YAML Complexity**: While file size limits provide basic protection, deeply nested YAML structures could theoretically cause high memory usage within the 1 MiB limit.

### Safe Error Handling Patterns

The codebase follows these error handling patterns to maintain security:

1. **Graceful Degradation**: Parsing errors skip the problematic file rather than crashing
2. **No Sensitive Data in Errors**: Error messages avoid exposing file contents or internal state
3. **UTF-8 Boundary Safety**: Fix application validates UTF-8 character boundaries before modifying content
4. **Bounded Iteration**: Regex matches and file walks use limits to prevent resource exhaustion
5. **Early Validation**: Invalid inputs are rejected at parsing stage before deeper processing

## Telemetry

agnix includes **opt-in** telemetry to help improve the tool. Telemetry is disabled by default.

### Privacy Guarantees

When telemetry is enabled, we collect only aggregate statistics:

**What we collect:**
- File type counts (e.g., "5 skills, 2 MCP configs") - NOT file paths or names
- Rule trigger counts (e.g., "AS-001: 3 times") - NOT diagnostic messages
- Error/warning/info counts
- Validation duration
- Random installation ID (not tied to user identity)

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

Security fixes are released as patch versions (e.g., 0.1.1) and announced in:
- GitHub Releases
- CHANGELOG.md
