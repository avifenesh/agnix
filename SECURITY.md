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
- **No network access**: agnix does not make network requests

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

## Security Updates

Security fixes are released as patch versions (e.g., 0.1.1) and announced in:
- GitHub Releases
- CHANGELOG.md
