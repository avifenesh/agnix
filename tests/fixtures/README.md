# Fixtures

This directory contains fixture files used by unit and CLI integration tests.
Keep fixtures minimal, deterministic, and focused on one rule family when possible.

## Conventions
- Use `tests/fixtures/valid` and `tests/fixtures/invalid` for general file-type fixtures.
- Use family-specific directories for rule-family coverage and cross-platform cases.
- Prefer filenames that hint at the rule or scenario (`xml-001-unclosed.md`, `pe-002-cot-on-simple.md`).
- Valid fixtures should avoid triggering diagnostics for their own family.

## Rule Family Coverage
| Family | Directory | Valid example | Invalid example(s) |
| --- | --- | --- | --- |
| AGM | `agents_md/` | `agents_md/valid/AGENTS.md` | `agents_md/no-headers/AGENTS.md` |
| XP | `cross_platform/` | `cross_platform/valid/AGENTS.md` | `cross_platform/hard-coded/AGENTS.md` |
| MCP | `mcp/` | `mcp/valid-tool.mcp.json` | `mcp/invalid-jsonrpc-version.mcp.json`, `mcp/missing-required-fields.mcp.json` |
| PE | `prompt/` | `prompt/pe-001-valid.md` | `prompt/pe-001-critical-in-middle.md` |
| REF | `refs/` | `refs/valid-links.md` | `refs/broken-link.md`, `refs/missing-import.md` |
| XML | `xml/` | `xml/xml-valid.md` | `xml/xml-001-unclosed.md`, `xml/xml-002-mismatch.md`, `xml/xml-003-unmatched.md` |

## Notes
- AGENTS.md and cross-platform fixtures intentionally overlap; they are validated by different rule families.
- Keep fixture paths stable, as tests assert on filenames.