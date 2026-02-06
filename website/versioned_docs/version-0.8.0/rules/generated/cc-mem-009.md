---
id: cc-mem-009
title: "CC-MEM-009 Token Count Exceeded"
sidebar_label: "CC-MEM-009"
---

## Summary

- **Rule ID**: `CC-MEM-009`
- **Severity**: `MEDIUM`
- **Category**: `Claude Memory`
- **Normative Level**: `SHOULD`
- **Verified On**: `2026-02-04`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://code.claude.com/docs/en/memory

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples are illustrative snippets for this rule category.

### Invalid

```markdown
# Memory
Always be helpful.
```

### Valid

```markdown
# Project Memory
- Use Rust workspace conventions
- Keep AGENTS.md and CLAUDE.md identical
```
