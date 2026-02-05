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
| AS, CC-SK | `valid/skills/`, `invalid/skills/` | `valid/skills/code-review/SKILL.md` | `invalid/skills/unknown-tool/SKILL.md` |
| CC-HK | `valid/hooks/`, `invalid/hooks/` | `valid/hooks/valid-hooks.json` | `invalid/hooks/` fixtures |
| CC-AG | `valid/agents/`, `invalid/agents/` | `valid/agents/valid-agent.md` | `invalid/agents/missing-name.md` |
| CC-MEM | `valid/memory/`, `invalid/memory/` | `valid/memory/CLAUDE.md` | `invalid/memory/CLAUDE.md` |
| CC-PL | `valid/plugins/`, `invalid/plugins/` | `valid/plugins/` fixtures | `invalid/plugins/` fixtures |
| AGM | `agents_md/` | `agents_md/valid/AGENTS.md` | `agents_md/no-headers/AGENTS.md` |
| COP | `copilot/`, `copilot-invalid/` | `copilot/.github/copilot-instructions.md` | `copilot-invalid/.github/copilot-instructions.md` |
| CUR | `cursor/`, `cursor-invalid/`, `cursor-legacy/` | `cursor/.cursor/rules/valid.mdc` | `cursor-invalid/.cursor/rules/empty.mdc` |
| XP | `cross_platform/` | `cross_platform/valid/AGENTS.md` | `cross_platform/hard-coded/AGENTS.md` |
| MCP | `mcp/` | `mcp/valid-tool.mcp.json` | `mcp/invalid-jsonrpc-version.mcp.json` |
| PE | `prompt/` | `prompt/pe-001-valid.md` | `prompt/pe-001-critical-in-middle.md` |
| REF | `refs/` | `refs/valid-links.md` | `refs/broken-link.md`, `refs/missing-import.md` |
| XML | `xml/` | `xml/xml-valid.md` | `xml/xml-001-unclosed.md` |

## Notes
- AGENTS.md and cross-platform fixtures intentionally overlap; they are validated by different rule families.
- Keep fixture paths stable, as tests assert on filenames.